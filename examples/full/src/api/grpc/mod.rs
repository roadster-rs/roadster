pub mod hello_world;

use crate::api::grpc::hello_world::MyGreeter;
use crate::api::grpc::hello_world::greeter_server::GreeterServer;
use crate::app_state::AppState;
use tonic::transport::Server;
use tonic::transport::server::Router;

pub fn routes(_state: &AppState) -> anyhow::Result<Router> {
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(hello_world::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    let router = Server::builder()
        .add_service(reflection_service)
        .add_service(GreeterServer::new(MyGreeter));

    Ok(router)
}
