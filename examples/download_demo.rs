//! Example: Model download from HuggingFace, Ollama, and URLs
//!
//! Run with: cargo run --example download_demo

use ironvault::download::{ModelDownloader, ModelSource};

fn main() -> ironvault::Result<()> {
    println!("=== IronVault Download Example ===\n");

    // 1. Parse different source formats
    println!("1. Parsing model source URIs...");

    let hf_source = ModelSource::parse("hf://TheBloke/Llama-2-7B-GGUF/llama-2-7b.Q4_K_M.gguf");
    println!("   HuggingFace: {:?}", hf_source.is_ok());

    let ollama_source = ModelSource::parse("ollama://llama2:7b");
    println!("   Ollama: {:?}", ollama_source.is_ok());

    let url_source = ModelSource::parse("https://example.com/model.safetensors");
    println!("   URL: {:?}\n", url_source.is_ok());

    // 2. Create a downloader
    println!("2. Creating model downloader...");
    let _downloader = ModelDownloader::new("./downloaded_models");
    println!("   ✓ Downloader configured for ./downloaded_models\n");

    // 3. Create a downloader with HuggingFace token
    println!("3. Creating authenticated downloader...");
    let _authed_downloader =
        ModelDownloader::new("./downloaded_models").with_hf_token("hf_example_token".to_string());
    println!("   ✓ Token configured for private repos\n");

    // 4. Demonstrate download (dry run - would need network)
    println!("4. Download workflow (demonstration):");
    println!("   iv pull hf://user/repo/model.safetensors --store --name my-model");
    println!("   → Downloads file with SHA-256 verification");
    println!("   → Auto-stores in vault with name 'my-model'");
    println!("   → Only HTTPS URLs accepted (security)\n");

    // 5. Show source parsing details
    println!("5. Source format examples:");
    println!("   hf://{{org}}/{{repo}}/{{file}}     → HuggingFace Hub");
    println!("   ollama://{{model}}:{{tag}}        → Ollama Registry");
    println!("   https://example.com/model.bin → Direct URL download\n");

    println!("=== Download example complete ===");
    Ok(())
}
