#[allow(dead_code)]
pub trait Command{
    
    fn execute(&self);
    const OPTIONS: Vec<String>;
}