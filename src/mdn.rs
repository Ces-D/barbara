use std::io::{BufReader, Cursor};

use crate::url_entry::{SiteMapElement, UrlEntry, UrlEntryBuilder};
use flate2::read::GzDecoder;
use reqwest::Result;
use select::{
    document::Document,
    predicate::{And, Attr, Name, Or},
};
use xml::reader::XmlEvent;

const MDN_SITE_MAP_URL: &str = "https://developer.mozilla.org/sitemaps/en-us/sitemap.xml.gz";

pub async fn request_site_map() -> Result<Vec<UrlEntry>> {
    let client = reqwest::Client::new();
    let response = client.get(MDN_SITE_MAP_URL).send().await?.bytes().await?;
    let gz_decoder = GzDecoder::new(response.as_ref());
    let reader = BufReader::new(gz_decoder);

    let mut site_map: Vec<UrlEntry> = vec![];

    let parser = xml::EventReader::new(reader);
    let mut url_builder = UrlEntryBuilder::default();

    for e in parser {
        match e {
            Ok(event) => match event {
                XmlEvent::StartElement { name, .. } => {
                    let local_name = name.local_name;
                    if local_name == "url" {
                        url_builder.set_element(SiteMapElement::Url);
                    } else if local_name == "loc" {
                        url_builder.set_element(SiteMapElement::Loc);
                    } else if local_name == "lastmod" {
                        url_builder.set_element(SiteMapElement::Lastmod);
                    }
                }
                XmlEvent::EndElement { name } => {
                    let local_name = name.local_name;
                    if local_name == "url" {
                        url_builder.set_element(SiteMapElement::Url);
                        if let Ok(entry) = url_builder.build() {
                            site_map.push(entry);
                        } else {
                            println!("Error building URL entry");
                        }
                    }
                }
                XmlEvent::Characters(text) => {
                    let trimmed_text = text.trim().to_string();
                    if !trimmed_text.is_empty() {
                        url_builder.set_text(trimmed_text);
                    }
                }
                _ => continue,
            },
            Err(e) => println!("Error: {}", e),
        }
    }

    Ok(site_map)
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

pub async fn request_page(url: &str) -> Result<PageContent> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?.bytes().await?;
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
