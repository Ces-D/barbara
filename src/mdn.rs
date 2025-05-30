use std::io::{BufReader, Cursor};

use crate::{
    cache::{read_cache_async, write_to_cache_async},
    url_entry::{SiteMapElement, UrlEntry, UrlEntryBuilder},
};
use flate2::read::GzDecoder;
use reqwest::Result;
use select::{
    document::Document,
    predicate::{And, Attr, Name, Or},
};
use xml::reader::XmlEvent;

const MDN_SITE_MAP_URL: &str = "https://developer.mozilla.org/sitemaps/en-us/sitemap.xml.gz";
/// All MDN docs start with this URL
pub const BASE_URL: &str = "https://developer.mozilla.org/en-US/";

// These are URLs or base URLs that we want to ignore when parsing the sitemap
const IGNORE_URLS: [&str; 9] = [
    "https://developer.mozilla.org/en-US/plus/**",
    "https://developer.mozilla.org/en-US/curriculum/**",
    "https://developer.mozilla.org/en-US/play/**",
    "https://developer.mozilla.org/en-US/observatory/**",
    "https://developer.mozilla.org/en-US/",
    "https://developer.mozilla.org/en-US/404",
    "https://developer.mozilla.org/en-US/about",
    "https://developer.mozilla.org/en-US/advertising",
    "https://developer.mozilla.org/en-US/blog/",
];

fn path_is_ignored(path: &str) -> bool {
    let mut is_ignored = false;
    for ignore_path in IGNORE_URLS.iter() {
        if is_ignored {
            break;
        } else {
            if ignore_path.ends_with("**") {
                let ignore_path = &ignore_path[..ignore_path.len() - 2];
                is_ignored = path.starts_with(ignore_path);
            } else {
                is_ignored = path == *ignore_path;
            }
        }
    }
    is_ignored
}

/// Requests the MDN site map or reads from the cache if available.
pub async fn request_site_map(no_cache: bool) -> Result<Vec<UrlEntry>> {
    let mut cache = None;
    if !no_cache {
        cache = read_cache_async().await.unwrap_or(None);
    }
    match cache {
        Some(data) => {
            println!("Cache found, using it");
            Ok(data)
        }
        None => {
            let client = reqwest::Client::new();
            let response = client.get(MDN_SITE_MAP_URL).send().await?.bytes().await?;
            let gz_decoder = GzDecoder::new(response.as_ref());
            let reader = BufReader::new(gz_decoder);

            let mut site_map: Vec<UrlEntry> = vec![];

            let parser = xml::EventReader::new(reader);
            let mut path_builder = UrlEntryBuilder::default();

            for e in parser {
                match e {
                    Ok(event) => match event {
                        XmlEvent::StartElement { name, .. } => {
                            let local_name = name.local_name;
                            if local_name == "path" {
                                path_builder.set_element(SiteMapElement::Url);
                            } else if local_name == "loc" {
                                path_builder.set_element(SiteMapElement::Loc);
                            } else if local_name == "lastmod" {
                                path_builder.set_element(SiteMapElement::Lastmod);
                            }
                        }
                        XmlEvent::EndElement { name } => {
                            let local_name = name.local_name;
                            if local_name == "url" {
                                path_builder.set_element(SiteMapElement::Url);
                                match path_builder.build() {
                                    Ok(entry) => site_map.push(entry),
                                    Err(e) => match e {
                                        crate::url_entry::UrlEntryBuilderError::MissingLoc => continue, // This is most likely due to the url being ignored
                                        crate::url_entry::UrlEntryBuilderError::MissingClosingTag => println!("Missing closing tag"),
                                    },
                                }
                            }
                        }
                        XmlEvent::Characters(text) => {
                            let trimmed_text = text.trim().to_string();
                            if !trimmed_text.is_empty() {
                                if path_is_ignored(&trimmed_text) {
                                    path_builder.reset();
                                } else {
                                    path_builder.set_text(trimmed_text);
                                }
                            }
                        }
                        _ => continue,
                    },
                    Err(e) => println!("Error: {}", e),
                }
            }
            let cloned_site_map = site_map.clone();
            if !no_cache {
                match write_to_cache_async(cloned_site_map).await {
                    Ok(_) => println!("Cache written successfully"),
                    Err(_) => println!("Error writing cache"),
                }
            }

            Ok(site_map)
        }
    }
}

pub struct PageContent {
    pub title: String,
    pub description: String,
    pub headers: Vec<Header>,
}

impl std::fmt::Display for PageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        static PADDING: u8 = 5;
        writeln!(f, "Title: {}", self.title)?;
        writeln!(f, "Description: {}\n\n", self.description)?;
        writeln!(f, "Headers:")?;
        for header in &self.headers {
            let indent = " ".repeat((PADDING * header.level) as usize);
            writeln!(f, "{}|-- {}", indent, header.value)?;
        }

        Ok(())
    }
}

pub struct Header {
    pub level: u8,
    pub value: String,
}

/// Requests the page html from the given URL, parses it, and condenses the information into a struct.
/// The
pub async fn request_page(path: &str) -> Result<PageContent> {
    let client = reqwest::Client::new();
    let response = client.get(path).send().await?.bytes().await?;
    let reader = BufReader::new(Cursor::new(response));
    let document = Document::from_read(reader).unwrap();

    let mut title = String::new();
    let mut desc = String::new();
    let mut headers: Vec<Header> = vec![];

    for element in document.find(Or(Name("head"), And(Name("main"), Attr("id", "content")))) {
        if element.name() == Some("head") {
            for node in element.find(And(Attr("content", ()), Attr("name", "description"))) {
                if let Some(description) = node.attr("content") {
                    desc = description.trim().to_string();
                } else {
                    println!("No description found");
                }
            }

            for node in element.find(Name("title")) {
                title = node.text().trim().to_string();
            }
        } else if element.name() == Some("main") {
            for node in element.find(Or(Or(Name("h1"), Name("h2")), Name("h3"))) {
                headers.push(Header {
                    level: match node.name() {
                        Some(name) => match name {
                            "h1" => 1,
                            "h2" => 2,
                            "h3" => 3,
                            _ => 0,
                        },
                        None => 0,
                    },
                    value: node.text().trim().to_string(),
                });
            }
        }
    }

    Ok(PageContent {
        title,
        description: desc,
        headers,
    })
}
