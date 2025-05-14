use clap::{Parser, Subcommand};

mod fuzzy;
mod mdn;
mod url_entry;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fuzzy { open_in_browser } => {
            // Call the function to request the site map
            match mdn::request_site_map().await {
                Ok(entries) => {
                    let selection = fuzzy::fuzzy_search(
                        entries.iter().map(|entry| entry.loc.clone()).collect(),
                    )
                    .unwrap();
                    if open_in_browser {
                        webbrowser::open(selection.as_str()).unwrap();
                    } else {
                        println!("{}", selection);
                    }
                }
                Err(e) => eprintln!("Error fetching site map: {}", e),
            }
        }
        Commands::Preview { item } => {
            // Call the function to request the page content
            match mdn::request_page(&item).await {
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
            default_value = "true",
            help = "Open the selected url in the browser"
        )]
        open_in_browser: bool,
    },
    #[command(about = "Generates a summary of a MDN page. Useful for quick reference.")]
    Preview { item: String },
}
