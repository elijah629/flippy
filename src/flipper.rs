use anyhow::{Result, bail};
use cliclack::select;
use flipper_rpc::transport::serial::{list_flipper_ports, rpc::SerialRpcTransport};

pub fn pick_cli() -> Result<SerialRpcTransport> {
    let ports = list_flipper_ports()?;

    let port = match ports.len() {
        0 => bail!(
            "No flippers are connected currently, please try replugging them, then re-running this command."
        ),
        1 => &ports[0].port_name,
        _ => {
            let items: Vec<_> = ports
                .iter()
                .map(|x| (&x.port_name, &x.device_name, &x.port_name))
                .collect();

            select("Which device is the target Flipper Zero?")
                .items(&items)
                .interact()?
        }
    };

    let cli = SerialRpcTransport::new(port)?;

    Ok(cli)
}
