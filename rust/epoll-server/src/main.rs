

use crate::server::EpollServer;


pub mod server;




fn main() {
   let server = EpollServer::new();
   server.run_server();
}

