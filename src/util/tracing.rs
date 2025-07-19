use tracing::{Value, field};

pub fn optional_trace_field<T>(value: Option<T>) -> Box<dyn Value>
where
    T: ToString,
{
    value
        .map(|x| Box::new(field::display(x.to_string())) as Box<dyn Value>)
        .unwrap_or(Box::new(field::Empty))
}
