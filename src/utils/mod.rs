#[derive(Debug, PartialEq)]
pub struct MacAddr {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
}

impl MacAddr {
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> Self {
        Self {
            a: a,
            b: b,
            c: c,
            d: d,
            e: e,
            f: f,
        }
    }

    pub fn from(data: &str) -> Self {
        Self::new(0x00, 0x00, 0x00, 0x00, 0x00, 0x00)
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::MacAddr;

    #[test]
    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    fn it_should_convert_from_string() {
        assert_eq!(MacAddr::from("00:00:ef:ab:da:06"), MacAddr {
            a: 0x00,
            b: 0x00,
            c: 0xef,
            d: 0xab,
            e: 0xda,
            f: 0x06,
        });
    }
}
