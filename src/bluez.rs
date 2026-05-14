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

/* For documentation, see
 * https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/doc/org.bluez.Adapter.rst
 * https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/doc/org.bluez.Device.rst
 */

use anyhow::Result;
use evdev::KeyCode;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
};
use zbus::{
    Connection, MatchRule, MessageStream, Proxy,
    fdo::ObjectManagerProxy,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

use futures_lite::stream::StreamExt;

use crate::Status;

pub struct Bluez {
    adapter: Proxy<'static>,
    device_paired: bool,
    props: Proxy<'static>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceKind {
    Headset,    /* 0x0404 */
    Speaker,    /* 0x0408 */
    Headphones, /* 0x0410 */
    Portable,   /* 0x0418 */
    Car,        /* 0x0420 */
    Unknown,
}

pub struct Device {
    pub path: OwnedObjectPath,
    pub name: String,
    pub address: String,
    pub class: u32,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub rssi: i16,
    pub kind: DeviceKind,
}

impl Bluez {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::system().await?;
        let (adapter_path, device_path) = Self::get_adapter_and_device_path(&conn).await?;

        match adapter_path {
            Some(adapter_path) => {
                let adapter =
                    Proxy::new_owned(conn, "org.bluez", adapter_path, "org.bluez.Adapter1").await?;

                let props = Proxy::new(
                    adapter.connection(),
                    "org.bluez",
                    adapter.path(),
                    "org.freedesktop.DBus.Properties",
                )
                .await?;

                Ok(Self {
                    adapter,
                    device_paired: device_path.is_some(),
                    props,
                })
            }
            None => Err("No default adapter".into()),
        }
    }

    async fn set_powered(&self, power: bool) -> Result<(), Box<dyn std::error::Error>> {
        self.props
            .call_method(
                "Set",
                &("org.bluez.Adapter1", "Powered", Value::from(power)),
            )
            .await?;
        Ok(())
    }

    async fn set_pairable(&self, pair: bool) -> Result<(), Box<dyn std::error::Error>> {
        let adapter_props = Proxy::new(
            self.adapter.connection(),
            "org.bluez",
            "/org/bluez/hci0",
            "org.freedesktop.DBus.Properties",
        )
        .await?;

        adapter_props
            .call_method(
                "Set",
                &("org.bluez.Adapter1", "Pairable", Value::from(pair)),
            )
            .await?;

        Ok(())
    }

    async fn start_scan(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.adapter.call_method("StartDiscovery", &()).await?;
        Ok(())
    }

    async fn stop_scan(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.adapter.call_method("StopDiscovery", &()).await?;
        Ok(())
    }

    pub async fn connect(
        &self,
        device_path: &OwnedObjectPath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_powered(true).await?;
        let _ = self.stop_scan().await;

        let device = Proxy::new(
            self.adapter.connection(),
            "org.bluez",
            device_path,
            "org.bluez.Device1",
        )
        .await?;

        let props = Proxy::new(
            self.adapter.connection(),
            "org.bluez",
            device_path,
            "org.freedesktop.DBus.Properties",
        )
        .await?;

        let paired: bool = props
            .call_method("Get", &("org.bluez.Device1", "Paired"))
            .await
            .and_then(|r| r.body().deserialize())
            .unwrap_or(false);

        if !paired {
            self.set_pairable(true).await?;
            device.call_method("Pair", &()).await?;
            props
                .call_method("Set", &("org.bluez.Device1", "Trusted", Value::from(true)))
                .await?;
        }

        device.call_method("Connect", &()).await?;

        Ok(())
    }

    pub async fn scan_audio_devices(
        &self,
        tx: Sender<(KeyCode, Option<Status>, bool, Option<Device>)>,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_powered(true).await?;

        let rule_added = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender("org.bluez")?
            .interface("org.freedesktop.DBus.ObjectManager")?
            .member("InterfacesAdded")?
            .build();
        let rule_changed = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender("org.bluez")?
            .interface("org.freedesktop.DBus.Properties")?
            .member("PropertiesChanged")?
            .build();

        let mut stream_added =
            MessageStream::for_match_rule(rule_added, self.adapter.connection(), None).await?;
        let mut stream_changed =
            MessageStream::for_match_rule(rule_changed, self.adapter.connection(), None).await?;

        let mut known: HashMap<OwnedObjectPath, HashMap<String, OwnedValue>> = HashMap::new();

        self.start_scan().await?;

        enum Event {
            Added(OwnedObjectPath, HashMap<String, OwnedValue>),
            Changed(OwnedObjectPath, HashMap<String, OwnedValue>),
            Cancelled,
            None,
        }

        while !cancel.load(Ordering::Relaxed) {
            let event = futures_lite::future::or(
                futures_lite::future::or(
                    async {
                        if let Ok(Some(msg)) = stream_added.try_next().await {
                            if let Ok((path, ifaces)) = msg.body().deserialize::<(
                                OwnedObjectPath,
                                HashMap<String, HashMap<String, OwnedValue>>,
                            )>() {
                                if let Some(props) = ifaces.get("org.bluez.Device1") {
                                    return Event::Added(path, props.clone());
                                }
                            }
                        }
                        Event::None
                    },
                    async {
                        if let Ok(Some(msg)) = stream_changed.try_next().await {
                            let header = msg.header();
                            let Some(path) = header.path().map(|p| p.to_owned()) else {
                                return Event::None;
                            };
                            if let Ok((iface, changed, _)) = msg.body().deserialize::<(
                                String,
                                HashMap<String, OwnedValue>,
                                Vec<String>,
                            )>(
                            ) {
                                if iface == "org.bluez.Device1" {
                                    return Event::Changed(path.into(), changed);
                                }
                            }
                        }
                        Event::None
                    },
                ),
                async {
                    loop {
                        if cancel.load(Ordering::Relaxed) {
                            return Event::Cancelled;
                        }
                        async_io::Timer::after(std::time::Duration::from_millis(100)).await;
                    }
                },
            )
            .await;

            match event {
                Event::Added(path, props) => {
                    known.entry(path).or_default().extend(props);
                }
                Event::Changed(path, props) => {
                    known.entry(path).or_default().extend(props);
                }
                Event::Cancelled => break,
                Event::None => {}
            }

            for (path, props) in &known {
                if let Some(device) = Self::parse_audio_device(path, props) {
                    let _ = tx.send((KeyCode::KEY_BLUETOOTH, None, false, Some(device)));
                }
            }
        }

        self.stop_scan().await?;

        Ok(())
    }

    pub async fn remove_all_devices(&self) -> Result<(), Box<dyn std::error::Error>> {
        let proxy = ObjectManagerProxy::builder(self.adapter.connection())
            .destination("org.bluez")?
            .path("/")?
            .build()
            .await?;

        let objects = proxy.get_managed_objects().await?;

        for (path, ifaces) in &objects {
            if ifaces.contains_key("org.bluez.Device1") {
                self.adapter
                    .call_method("RemoveDevice", &(path.as_ref()))
                    .await
                    .ok(); // ignorer les erreurs individuelles
            }
        }

        Ok(())
    }

    fn parse_audio_device(
        path: &OwnedObjectPath,
        props: &HashMap<String, OwnedValue>,
    ) -> Option<Device> {
        const AUDIO_UUIDS: &[&str] = &[
            // "00001108-0000-1000-8000-00805f9b34fb", /* HSP       */
            // "0000111e-0000-1000-8000-00805f9b34fb", /* HFP       */
            "0000110b-0000-1000-8000-00805f9b34fb", /* A2DP Sink */
        ];

        let name = props
            .get("Name")
            .and_then(|v| String::try_from(v.clone()).ok())
            .unwrap_or_default();
        let class = props
            .get("Class")
            .and_then(|v| u32::try_from(v.clone()).ok())
            .unwrap_or_default();
        let uuids: Vec<String> = props
            .get("UUIDs")
            .and_then(|v| Vec::<String>::try_from(v.clone()).ok())
            .unwrap_or_default();

        println!("BT: {}, CLASS={:x}, UUIDs={:?}", name, class, uuids);

        let kind = match class & 0x1FFF {
            0x0404 => DeviceKind::Headset,
            0x0408 => DeviceKind::Speaker,
            0x0410 => DeviceKind::Headphones,
            0x0418 => DeviceKind::Portable,
            0x0420 => DeviceKind::Car,
            _ => DeviceKind::Unknown,
        };

        /* Only audio devices */
        if !uuids.iter().any(|u| AUDIO_UUIDS.contains(&u.as_str())) {
            if kind == DeviceKind::Unknown {
                return None;
            }
        }

        let rssi = props
            .get("RSSI")
            .and_then(|v| i16::try_from(v.clone()).ok())
            .unwrap_or_default();

        println!("BT: {}, RSSI={}", name, rssi);

        /* Only reachable devices */
        if rssi == 0 {
            return None;
        }

        let address = props
            .get("Address")
            .and_then(|v| String::try_from(v.clone()).ok())
            .unwrap_or_default();
        let paired = props
            .get("Paired")
            .and_then(|v| bool::try_from(v.clone()).ok())
            .unwrap_or_default();
        let connected = props
            .get("Connected")
            .and_then(|v| bool::try_from(v.clone()).ok())
            .unwrap_or_default();
        let trusted = props
            .get("Trusted")
            .and_then(|v| bool::try_from(v.clone()).ok())
            .unwrap_or_default();

        Some(Device {
            path: path.clone(),
            name,
            address,
            class,
            paired,
            connected,
            trusted,
            rssi,
            kind,
        })
    }

    pub fn get_device_paired(&self) -> bool {
        self.device_paired
    }

    async fn get_adapter_and_device_path(
        conn: &Connection,
    ) -> Result<(Option<OwnedObjectPath>, Option<OwnedObjectPath>), Box<dyn std::error::Error>>
    {
        let proxy = ObjectManagerProxy::builder(conn)
            .destination("org.bluez")?
            .path("/")?
            .build()
            .await?;

        let objects = proxy.get_managed_objects().await?;
        let mut device_path: Option<OwnedObjectPath> = None;
        let mut adapter_path: Option<OwnedObjectPath> = None;

        for (path, ifaces) in &objects {
            if let Some(_props) = ifaces.get("org.bluez.Adapter1") {
                adapter_path = Some(path.clone());
            }
            if let Some(props) = ifaces.get("org.bluez.Device1") {
                let paired = props
                    .get("Paired")
                    .and_then(|v| bool::try_from(v.clone()).ok())
                    .unwrap_or(false);
                let trusted = props
                    .get("Trusted")
                    .and_then(|v| bool::try_from(v.clone()).ok())
                    .unwrap_or(false);

                if !paired || !trusted {
                    continue;
                }

                device_path = Some(path.clone());
            }
        }

        Ok((adapter_path, device_path))
    }
}
