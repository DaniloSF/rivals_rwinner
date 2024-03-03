mod config;

use dll_syringe::{
    process::{OwnedProcess, Process},
    Syringe,
};

use std::{
    env,
    io::{copy, Read, Stdout, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::OnceLock,
};
use tracing::info;

static DATA_SENDER: OnceLock<TcpStream> = OnceLock::new();

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt().init();
    info!("Hello, world!");

    let target_process = OwnedProcess::find_first_by_name("RivalsofAether.exe")
        .expect("RivalsofAether.exe not found!");
    info!("Found process: {:?}", target_process.base_name());

    let debug_listener = TcpListener::bind(config::get_debug_address())?; // Bind debug to the local address
    let data_listener = TcpListener::bind(config::get_data_address())?; // Bind data to the local address
    let data_sender_stream = TcpStream::connect(config::get_send_address());
    match data_sender_stream {
        Ok(stream) => {
            DATA_SENDER.set(stream).unwrap();
        }
        Err(e) => {
            info!("Failed to connect to data sender: {}, continuing...", e);
        }
    }

    let syringe = Syringe::for_process(target_process);
    info!("Injecting...");

    // Get dll path, it resides in the same directory as the executable, use relative path
    let dll_path = Path::parent(env::current_exe().unwrap().as_path())
        .unwrap()
        .join("rivals_rwinner.dll");

    info!("Injecting DLL: {:?}", dll_path);
    let _injected_payload = syringe.inject(dll_path).unwrap();
    info!("Injected!");

    let (mut debug_stream, debug_addr) = debug_listener.accept()?; // Accept the incoming connection
    info!("Accepted debug connection from: {}", debug_addr);

    let (mut data_stream, data_addr) = data_listener.accept()?; // Accept the incoming connection
    info!("Accepted data connection from: {}", data_addr);

    let mut debug_stdout = std::io::stdout();
    let mut cloned_debug_stream = debug_stream
        .try_clone()
        .expect("Failed to clone debug_stream");
    let mut cloned_data_stream = data_stream
        .try_clone()
        .expect("Failed to clone data_stream");

    // Read the incoming data and send to stdout with std::io::copy in a separate thread
    let debug_thread =
        std::thread::spawn(move || read_debug(&mut cloned_debug_stream, &mut debug_stdout));
    let tcp_thread = std::thread::spawn(move || read_data(&mut cloned_data_stream));

    debug_thread.join().unwrap();
    tcp_thread.join().unwrap();

    syringe.eject(_injected_payload).unwrap();
    info!("Ejected!");

    debug_stream.shutdown(std::net::Shutdown::Both).unwrap();
    data_stream.shutdown(std::net::Shutdown::Both).unwrap();

    Ok(())
}

fn read_debug(stream: &mut TcpStream, stdout: &mut Stdout) {
    copy(stream, stdout).expect("Failed to read data from debug stream");
}

// read the data from the stream, convert bytes into f64 then to i32
fn read_data(stream: &mut TcpStream) {
    let mut buffer = [0; 8];
    while let Ok(n) = stream.read(&mut buffer) {
        let data = f64::from_ne_bytes(buffer);
        let player_winner = data as i32;

        info!("Player won: {}", player_winner);
        if DATA_SENDER.get().is_some() {
            send_data_ipc(player_winner);
        }
    }
}

fn send_data_ipc(player_won: i32) {
    DATA_SENDER
        .get()
        .unwrap()
        .write_all(&player_won.to_ne_bytes())
        .unwrap();
}
