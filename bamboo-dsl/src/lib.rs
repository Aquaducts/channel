#[derive(Debug)]
pub struct Step {
    pub name: Option<String>,
    pub run: Option<String>,
}

#[derive(Debug)]
pub struct Bamboo {
    pub steps: Vec<Step>,
}

pub enum Statements {
    AddStep,
    ArgumentBinding(String),
}

pub fn parse_config(input: String) -> Bamboo {
    let mut bamboo = Bamboo { steps: Vec::new() };
    let mut step: Step = Step {
        name: None,
        run: None,
    };
    let mut input_chars = input.chars().peekable();
    let mut current_stmt: Option<Statements> = None;

    while let Some(c) = input_chars.peek() {
        match c {
            // The beggining of a statement, most likely an `addStep` statement.
            '(' => {
                let mut statement = String::new();
                input_chars.next();
                while let Some(c) = input_chars.peek() {
                    if c == &' ' || c == &'\n' || c == &'\t' {
                        // Move it forward one so we dont get one of
                        // three characters above :D
                        input_chars.next();
                        break;
                    }
                    statement.push(*c);
                    input_chars.next();
                }
                if &statement == "addStep" {
                    current_stmt = Some(Statements::AddStep);
                }
                println!("Got statement: {statement}");
                input_chars.next();
            }
            ')' => {
                input_chars.next();
            }
            ':' => {
                // Cant be anything else ig
                input_chars.next();

                let mut name = String::new();
                // Get name
                while let Some(c) = input_chars.peek() {
                    if c == &' ' || c == &'\n' || c == &'\t' {
                        // Move it forward one so we dont get one of
                        // three characters above :D
                        input_chars.next();
                        break;
                    }
                    name.push(*c);
                    input_chars.next();
                }

                println!("Arg name: \"{name}\"");
                current_stmt = Some(Statements::ArgumentBinding(name));
            }
            '"' => {
                input_chars.next();
                // Handle the possible value of an uhm thingy!
                // if the current_stmt is not ArgumentBinding then just pass!
                if let Some(Statements::ArgumentBinding(which)) = &current_stmt {
                    let mut value = String::new();
                    // Get name
                    while let Some(c) = input_chars.peek() {
                        if c == &'"' {
                            input_chars.next();
                            break;
                        }
                        value.push(*c);
                        input_chars.next();
                    }
                    println!("Got value: \"{value}\"");
                    step = match which.as_str() {
                        "name" => Step {
                            name: Some(value),
                            ..step
                        },
                        "run" => Step {
                            run: Some(value),
                            ..step
                        },
                        _ => {
                            println!("{:?}", which);
                            unimplemented!()
                        }
                    };
                }
                input_chars.next();
            }
            _ => {
                input_chars.next();
            }
        }
    }

    bamboo.steps.push(step);

    bamboo
}
