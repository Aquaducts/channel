use crate::{
    lxc::{lxc_attach_options_t, lxc_container},
    make_str,
};
use anyhow::{bail, Result};
use std::ptr::null;

pub struct Command {
    pub program: String,
    pub args: Vec<String>,
}

impl TryInto<Command> for String {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Command, Self::Error> {
        let args = self.split_whitespace().collect::<Vec<&str>>().into_iter();

        let program = match &args.clone().rev().last() {
            Some(p) => p.to_owned(),
            None => bail!("Collected list doesn't have a head"),
        };

        Ok(Command {
            program: program.to_string(),
            args: args
                .to_owned()
                .map(|e| e.to_string())
                .collect::<Vec<String>>(),
        })
    }
}

impl Command {
    pub fn new<T: Into<String>, A: Into<Vec<String>>>(program: T, args: A) -> Self {
        Self {
            program: program.into(),
            args: args.into(),
        }
    }

    pub fn exec(
        &self,
        container: *mut lxc_container,
        attach_opts: &mut lxc_attach_options_t,
    ) -> i32 {
        let argv = self
            .args
            .iter()
            .map(|e| std::ffi::CString::new(e.as_str()).unwrap())
            .collect::<Vec<std::ffi::CString>>();

        let mut argv = argv.iter().map(|e| e.as_ptr()).collect::<Vec<*const i8>>();

        argv.push(null());
        println!("{:?}", self.program.as_str());

        unsafe {
            (*container).attach_run_wait.unwrap()(
                container,
                attach_opts,
                make_str!(self.program.as_str()),
                argv.as_ptr(),
            )
        }
    }
}
