/// An instance of the MDN site map
/// Url entry. Contains url location, modification time, priority, update frequency.
#[derive(Clone, Debug)]
pub struct UrlEntry {
    /// URL of the page.
    pub loc: String,
    /// The date of last modification of the file.
    pub lastmod: Option<String>,
}

#[derive(PartialEq)]
/// MDN site map elements are either url, loc, or lastmod.
pub enum SiteMapElement {
    /// The encapsulating element for a URL entry.
    Url,
    /// The URL location.
    Loc,
    /// The last modification date.
    Lastmod,
}

pub struct UrlEntryBuilder {
    element: SiteMapElement,
    loc: Option<String>,
    lastmod: Option<String>,
}

impl UrlEntryBuilder {
    pub fn set_element(&mut self, element: SiteMapElement) {
        self.element = element;
    }

    pub fn set_text(&mut self, text: String) {
        match self.element {
            SiteMapElement::Loc => {
                self.loc = Some(text);
            }
            SiteMapElement::Lastmod => {
                self.lastmod = Some(text);
            }
            _ => {}
        }
    }

    pub fn build(&self) -> Result<UrlEntry, UrlEntryBuilderError> {
        if self.element == SiteMapElement::Url {
            if self.loc.is_none() {
                Err(UrlEntryBuilderError::MissingLoc)
            } else {
                Ok(UrlEntry {
                    loc: self.loc.clone().expect("loc is required"),
                    lastmod: self.lastmod.clone(),
                })
            }
        } else {
            Err(UrlEntryBuilderError::MissingClosingTag)
        }
    }
}

impl Default for UrlEntryBuilder {
    fn default() -> Self {
        UrlEntryBuilder {
            element: SiteMapElement::Url,
            loc: None,
            lastmod: None,
        }
    }
}

pub enum UrlEntryBuilderError {
    MissingLoc,
    MissingClosingTag,
}
