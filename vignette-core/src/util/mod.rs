pub fn bool_parser(value: u8) -> bool {
    value != 0
}

pub fn bool_writer(value: &bool) -> u8 {
    if *value { 1 } else { 0 }
}
