use roadster::app_context::AppContext;

pub type CustomAppContext = ();

pub type AppState = AppContext<CustomAppContext>;
