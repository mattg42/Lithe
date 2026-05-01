pub trait Machine {
    fn set_silent(&mut self, to: bool);
    fn print_expression(&self);
    fn expand_expression(&mut self);
    fn output(&self) -> &[String];
    fn step(&mut self) -> StepResult;
    fn run(&mut self, trace: bool) -> StepResult;
}

#[derive(Debug, PartialEq)]
/// Represents the result of a machine transition step.
pub enum StepResult {
    Continue,
    Stop,
    Failure,
}

#[derive(PartialEq)]
pub enum MachineType {
    Stack,
    Krivine,
}
