// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use context::*;

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BusType {
    I2C,
    ISA,
    PCI,
    SPI,
    Virtual,
    ACPI,
    HID,
    MDIO,
}

#[derive(Clone)]
pub struct BusId {
    bus_type: BusType,
    bus_number: i16,
    context: Context,
}

impl BusId {
    pub fn new(bus_type: BusType, bus_number: i16, context: Context) -> BusId {
        BusId {
            bus_type,
            bus_number,
            context,
        }
    }

    pub fn get_type(&self) -> BusType {
        self.bus_type
    }

    pub fn number(&self) -> i16 {
        self.bus_number
    }

    pub fn adapter_name(&self) -> String {
        let mut name = match self.bus_type {
            BusType::ISA => "ISA adapter",
            BusType::PCI => "PCI adapter",
            // SPI should not be here, but for now SPI adapters have no name
            // so we don't have any custom string to return.
            BusType::SPI => "SPI adapter",
            BusType::Virtual => "Virtual device",
            BusType::ACPI => "ACPI interface",
            // HID should probably not be there either, but I don't know if
            // HID buses have a name nor where to find it.
            BusType::HID => "HID adapter",
            BusType::MDIO => "MDIO adapter",
            _ => "",
        }.to_string();

        // Bus types with several instances
        if name.is_empty() {
            let proc_bus = self.context.buses();
            for bus in proc_bus.iter() {
                if bus.get_type() == self.get_type() && bus.number() == self.number() {
                    name = bus.adapter().to_string();
                    break;
                }
            }
        }

        name
    }
}
