// SPDX-FileCopyrightText: 2019 Camille019
// SPDX-License-Identifier: MPL-2.0

use crate::ratio::Ratio;

#[allow(non_upper_case_globals, dead_code)]
pub const Unity: Ratio<u64> = Ratio::new_raw(1, 1);

pub mod si {
    #![allow(non_upper_case_globals, dead_code)]
    pub use crate::prefix::Unity;
    use crate::ratio::Ratio;

    pub const Yocto: Ratio<u128> = Ratio::new_raw(1, 1_000_000_000_000_000_000_000_000);
    pub const Zepto: Ratio<u128> = Ratio::new_raw(1, 1_000_000_000_000_000_000_000);
    pub const Atto: Ratio<u64> = Ratio::new_raw(1, 1_000_000_000_000_000_000);
    pub const Femto: Ratio<u64> = Ratio::new_raw(1, 1_000_000_000_000_000);
    pub const Pico: Ratio<u64> = Ratio::new_raw(1, 1_000_000_000_000);
    pub const Nano: Ratio<u64> = Ratio::new_raw(1, 1_000_000_000);
    pub const Micro: Ratio<u64> = Ratio::new_raw(1, 1_000_000);
    pub const Milli: Ratio<u64> = Ratio::new_raw(1, 1_000);
    pub const Centi: Ratio<u64> = Ratio::new_raw(1, 100);
    pub const Deci: Ratio<u64> = Ratio::new_raw(1, 10);
    pub const Deca: Ratio<u64> = Ratio::new_raw(10, 1);
    pub const Hecto: Ratio<u64> = Ratio::new_raw(100, 1);
    pub const Kilo: Ratio<u64> = Ratio::new_raw(1_000, 1);
    pub const Mega: Ratio<u64> = Ratio::new_raw(1_000_000, 1);
    pub const Giga: Ratio<u64> = Ratio::new_raw(1_000_000_000, 1);
    pub const Tera: Ratio<u64> = Ratio::new_raw(1_000_000_000_000, 1);
    pub const Peta: Ratio<u64> = Ratio::new_raw(1_000_000_000_000_000, 1);
    pub const Exa: Ratio<u64> = Ratio::new_raw(1_000_000_000_000_000_000, 1);
    pub const Zetta: Ratio<u128> = Ratio::new_raw(1_000_000_000_000_000_000_000, 1);
    pub const Yotta: Ratio<u128> = Ratio::new_raw(1_000_000_000_000_000_000_000_000, 1);
}

pub mod iec {
    #![allow(non_upper_case_globals, dead_code)]
    pub use crate::prefix::Unity;
    use crate::ratio::Ratio;

    pub const Kibi: Ratio<u64> = Ratio::new_raw(1_024, 1);
    pub const Mebi: Ratio<u64> = Ratio::new_raw(1_048_576, 1);
    pub const Gibi: Ratio<u64> = Ratio::new_raw(1_073_741_824, 1);
    pub const Tebi: Ratio<u64> = Ratio::new_raw(1_099_511_627_776, 1);
    pub const Pebi: Ratio<u64> = Ratio::new_raw(1_125_899_906_842_624, 1);
    pub const Exbi: Ratio<u64> = Ratio::new_raw(1_152_921_504_606_846_976, 1);
    pub const Zebi: Ratio<u128> = Ratio::new_raw(1_180_591_620_717_411_303_424, 1);
    pub const Yobi: Ratio<u128> = Ratio::new_raw(1_208_925_819_614_629_174_706_176, 1);
}
