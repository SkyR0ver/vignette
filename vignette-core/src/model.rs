use crate::hid::HidDevInfo;

struct DevSpec {
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    usage_id: u16,
}

impl PartialEq<HidDevInfo> for DevSpec {
    fn eq(&self, other: &HidDevInfo) -> bool {
        self.vendor_id == other.vendor_id
            && self.product_id == other.product_id
            && self.usage_page == other.usage_page
            && self.usage_id == other.usage_id
    }
}

const SUPPORTED_DEVICES: &[DevSpec] = &[DevSpec {
    vendor_id: 0x320F,
    product_id: 0x5055,
    usage_page: 0xFF1C,
    usage_id: 0x92,
}];

pub fn is_supported(dev: &HidDevInfo) -> bool {
    SUPPORTED_DEVICES.iter().any(|spec| spec == dev)
}
