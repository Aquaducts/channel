use crate::{command::Command, lxc, make_str};
use anyhow::{bail, Result};
use std::ptr::{null, null_mut};

pub struct Container {
    pub name: String,
    inner: *mut lxc::lxc_container,
}

impl Container {
    pub fn new<T: Into<String>>(name: T) -> Result<Self> {
        unsafe {
            let name: String = name.into();
            let container = lxc::lxc_container_new(make_str!(name.as_str()), null());

            let true = (*container).createl.unwrap()(
                container,
                make_str!("download"),
                null(),
                null_mut(),
                lxc::LXC_CREATE_QUIET as i32,
                make_str!("-d"),
                make_str!("alpine"),
                make_str!("-r"),
                make_str!("edge"),
                make_str!("-a"),
                make_str!("amd64"),
                null() as *const i8,
            ) else {
                bail!("Failed to uhm ye.");
            };

            Ok(Self {
                name,
                inner: container,
            })
        }
    }

    pub fn start(&self) -> Result<()> {
        unsafe {
            if !(*self.inner).start.unwrap()(self.inner, 0, null_mut()) {
                bail!("Failed to start the container");
            }
            Ok(())
        }
    }

    pub fn exec(&self, command: Command, attach_opts: &mut lxc::lxc_attach_options_t) -> i32 {
        command.exec(self.inner, attach_opts)
    }

    pub fn stop(&self) -> Result<()> {
        unsafe {
            (*self.inner).stop.unwrap()(self.inner);
            Ok(())
        }
    }

    pub fn destroy(&self) -> Result<()> {
        unsafe {
            (*self.inner).destroy.unwrap()(self.inner);
            Ok(())
        }
    }
}
