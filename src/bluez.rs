/* Contelia
 * Copyright (C) 2026  Mathieu Schroeter <mathieu@schroetersa.ch>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use anyhow::Result;
use zbus::{
    Connection, Proxy,
    fdo::ObjectManagerProxy,
    zvariant::{OwnedObjectPath, Value},
};

pub struct Bluez {
    adapter: Proxy<'static>,
    props: Proxy<'static>,
}

impl Bluez {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::system().await?;
        let adapter_path = Self::get_default_adapter_path(&conn).await?;

        let adapter =
            Proxy::new_owned(conn, "org.bluez", adapter_path, "org.bluez.Adapter1").await?;

        let props = Proxy::new(
            adapter.connection(),
            "org.bluez",
            adapter.path(),
            "org.freedesktop.DBus.Properties",
        )
        .await?;

        Ok(Self { adapter, props })
    }

    pub async fn set_powered(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.props
            .call_method("Set", &("org.bluez.Adapter1", "Powered", Value::from(true)))
            .await?;
        Ok(())
    }

    pub async fn is_powered(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let powered = self
            .props
            .call_method("Get", &("org.bluez.Adapter1", "Powered"))
            .await?
            .body();
        let powered = powered.deserialize::<zbus::zvariant::Value>()?;
        Ok(bool::try_from(powered)?)
    }

    async fn get_default_adapter_path(
        conn: &Connection,
    ) -> Result<OwnedObjectPath, Box<dyn std::error::Error>> {
        let proxy = ObjectManagerProxy::builder(conn)
            .destination("org.bluez")?
            .path("/")?
            .build()
            .await?;

        let objects = proxy.get_managed_objects().await?;

        for (path, ifaces) in &objects {
            if let Some(props) = ifaces.get("org.bluez.Adapter1") {
                let name = props
                    .get("Name")
                    .and_then(|v| String::try_from(v.clone()).ok())
                    .unwrap_or_default();
                let address = props
                    .get("Address")
                    .and_then(|v| String::try_from(v.clone()).ok())
                    .unwrap_or_default();
                let powered = props
                    .get("Powered")
                    .and_then(|v| bool::try_from(v.clone()).ok())
                    .unwrap_or(false);
                println!("path    : {}", path);
                println!("name    : {}", name);
                println!("address : {}", address);
                println!("powered : {}", powered);
                return Ok(path.clone());
            }
        }

        Err("No default adapter".into())
    }
}
