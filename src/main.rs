use clap::{Parser, Subcommand};

mod cache;
mod fuzzy;
mod mdn;
mod url_entry;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    cache::spawn_cache_cleaner();

    match cli.command {
        Commands::Fuzzy {
            open_in_browser,
            no_cache,
        } => {
            // Call the function to request the site map
            match mdn::request_site_map(no_cache).await {
                Ok(entries) => {
                    let selection = fuzzy::fuzzy_search(
                        entries
                            .iter()
                            .map(|entry| entry.loc.replace(mdn::BASE_URL, ""))
                            .collect(),
                    )
                    .unwrap();
                    if open_in_browser {
                        let selection = format!("{}{}", mdn::BASE_URL, selection);
                        match open::that(selection.as_str()) {
                            Ok(_) => println!("Opening in browser: {}", selection),
                            Err(e) => println!("Error opening in browser: {}", e),
                        };
                    } else {
                        println!("{}", selection);
                    }
                }
                Err(e) => eprintln!("Error fetching site map: {}", e),
            }
        }
        Commands::Preview { item } => {
            // Call the function to request the page content
            let corrected_url = item
                .starts_with(mdn::BASE_URL)
                .then_some(item.to_string())
                .unwrap_or_else(|| format!("{}{}", mdn::BASE_URL, item));

            match mdn::request_page(&corrected_url).await {
                Ok(page_content) => {
                    println!("{}", page_content);
                }
                Err(e) => eprintln!("Error fetching page: {}", e),
            }
        }
    };
}

#[derive(Parser)]
#[command(about = "MDN Site Map Cli")]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Fuzzy search for a topic on the MDN website")]
    Fuzzy {
        #[arg(
            short,
            long,
            default_value = "false",
            help = "Open the selected url in the browser"
        )]
        open_in_browser: bool,
        #[arg(
            long,
            default_value = "false",
            help = "Dont read from the cache if it exists and don't write a cache file"
        )]
        no_cache: bool,
    },
    #[command(about = "Generates a summary of a MDN page. Useful for quick reference.")]
    Preview { item: String },
}
