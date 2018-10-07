extern crate env_logger;
extern crate hwmonlx;

fn main() {
    env_logger::init();

    let context = hwmonlx::Context::new(None).unwrap();

    match hwmonlx::read_sysfs_chips(&context) {
        Ok(chips) => {
            for chip in chips.iter() {
                println!("{}", chip.name());
                println!("Adapter: {}", chip.bus().adapter_name());
                for feature in chip.features_iter() {
                    println!("  - {}", feature.label());
                    for subfeature in feature.subfeatures_iter() {
                        let name = subfeature.name();
                        let value = subfeature.read_value();

                        match value {
                            Ok(value) => println!("    - {} = {}", name, value),
                            Err(e) => println!("    - {}: {:?}", name, e),
                        };
                    }
                }
                println!();
            }
        }
        Err(e) => println!("{:?}", e),
    }
}
