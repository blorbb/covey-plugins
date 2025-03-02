use std::io;

use covey_plugin::{
    List, ListItem, Menu, Plugin, Result,
    rank::{self, Weights},
};

covey_plugin::include_manifest!();

fn run_then_close(f: impl Fn() -> io::Result<()> + 'static) -> impl AsyncFn(Menu) -> Result<()> {
    async move |menu| {
        menu.close();
        Ok(f()?)
    }
}

struct SystemPower {
    items: Vec<ListItem>,
}

impl Plugin for SystemPower {
    type Config = ();

    async fn new(_config: Self::Config) -> Result<Self> {
        let items = vec![
            ListItem::new("sleep")
                .with_icon_name("system-suspend")
                .on_activate(run_then_close(system_shutdown::sleep)),
            ListItem::new("logout")
                .with_icon_name("system-log-out")
                .on_activate(run_then_close(system_shutdown::logout)),
            ListItem::new("shutdown")
                .with_icon_name("system-shutdown")
                .on_activate(run_then_close(system_shutdown::shutdown)),
            ListItem::new("restart")
                .with_icon_name("system-reboot")
                .on_activate(run_then_close(system_shutdown::reboot)),
            // hibernate is not implemented by system_shutdown on macos
            #[cfg(not(target_os = "macos"))]
            ListItem::new("hibernate")
                .with_icon_name("system-hibernate")
                .on_activate(run_then_close(system_shutdown::hibernate)),
        ];

        Ok(Self { items })
    }

    async fn query(&self, query: String) -> Result<List> {
        let sorted = rank::rank(&query, &self.items, Weights::with_history()).await;
        Ok(List::new(sorted))
    }
}

fn main() {
    covey_plugin::run_server::<SystemPower>(env!("CARGO_BIN_NAME"));
}
