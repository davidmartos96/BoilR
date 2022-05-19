#[derive(Default, Clone, Debug)]
pub struct CommandlineBuilder {
    environment_variables: Vec<String>,
    executable: String,
    parameters: Vec<String>,
    pre_parameters: Vec<String>,
}

impl CommandlineBuilder {
    pub fn add_environment_variable<S: AsRef<str>, S2: AsRef<str>>(&mut self, name: S, value: S2) {
        let string = format!("{}={}", name.as_ref().to_uppercase(), value.as_ref());
        self.environment_variables.push(string);
    }

    pub fn add_parameter<S: AsRef<str>>(&mut self, parameter: S) {
        self.parameters.push(parameter.as_ref().to_string());
    }

    pub fn add_pre_parameter<S: AsRef<str>>(&mut self, parameter: S) {
        self.pre_parameters.push(parameter.as_ref().to_string());
    }
    pub fn add_parameter_path<S: AsRef<str>>(&mut self, parameter: S) {
        self.parameters.push(format!("'{}'", parameter.as_ref()));
    }

    pub fn set_executable<S: AsRef<str>>(&mut self, executable: S) {
        self.executable = executable.as_ref().to_string();
    }

    pub fn build_command(&self) -> String {
        let mut result = String::new();
        let environment_variables_string = self.environment_variables.join(" ");
        let parameters_string = self.parameters.join(" ");
        let pre_parameters_string = self.pre_parameters.join(" ");
        format!(
            "{} {} {} {}",
            environment_variables_string, pre_parameters_string, self.executable, parameters_string
        )
    }
}
