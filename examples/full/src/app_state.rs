use roadster::app::context::AppContext;

pub type CustomAppContext = ();

pub type AppState = AppContext<CustomAppContext>;
