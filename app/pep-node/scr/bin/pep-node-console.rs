use pep_core::blockchain::network::core::Core;

fn main() {
    println!("====================================");
    println!("          PEP NETWORK NODE");
    println!("====================================");
    println!();
    println!("P2P        : 0.0.0.0:6000");
    println!("Discovery  : UDP 6001");
    println!();
    println!("Starting PEP Core...");
    println!();

    Core::start(
        "0.0.0.0:6000",
        None,
    );
}