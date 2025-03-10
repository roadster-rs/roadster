mod hello_world;

use crate::grpc::hello_world::MyGreeter;
use crate::grpc::hello_world::greeter_server::GreeterServer;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::service::grpc::service::GrpcService;
use tonic::transport::Server;
use tonic::transport::server::Router;

fn build_app() -> RoadsterResult<RoadsterApp<AppContext>> {
    let app = RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        // Register the gRPC service with the provided routes
        .add_service(GrpcService::new(routes()?))
        .build();
    Ok(app)
}

/// Build the gRPC [`Router`].
fn routes() -> RoadsterResult<Router> {
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(hello_world::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    let router = Server::builder()
        .add_service(reflection_service)
        .add_service(GreeterServer::new(MyGreeter));

    Ok(router)
}
