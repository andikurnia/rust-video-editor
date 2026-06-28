use clap::Parser;

#[derive(Parser)]
#[command(name = "video-editor", version, about = "Rust Video Editor")]
struct Cli {
    #[arg(long, help = "Start the MCP server for AI integration")]
    mcp: bool,

    #[arg(short, long, help = "Project file to open")]
    project: Option<String>,
}

fn run_gui() -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Rust Video Editor"),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Video Editor",
        options,
        Box::new(|_cc| Ok(Box::new(editor_ui::VideoEditorApp::new()))),
    )
    .map_err(|e| format!("{e:#}"))
}

fn run_mcp_server() {
    let server = mcp_server::McpServer::new();
    tracing::info!("MCP server started (stdio transport)");
    for line in std::io::stdin().lines() {
        match line {
            Ok(request) => match server.handle_request(&request) {
                Ok(response) => {
                    println!("{response}");
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                }
            },
            Err(e) => {
                eprintln!("stdin error: {e}");
                break;
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if cli.mcp {
        tracing::info!("Starting in MCP server mode");
        run_mcp_server();
    } else {
        tracing::info!("Starting GUI");
        if let Err(e) = run_gui() {
            eprintln!("GUI error: {e}");
        }
    }
}
