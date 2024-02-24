use hibp_core::minbitrep::MinBitRep;

#[test]
fn test_minbitrep() {

    let len = 8;
    let bit_len = 30u8;
    let value = (1<<bit_len-1)|1;

    let mut a = vec![0u8; MinBitRep::calculate_array_size(len, value)];
    let mut mbr = MinBitRep::wrap(a.as_mut_slice(), value);
    for i in 0..len {
        mbr.set(i, value);
        assert_eq!(mbr.get(i), value);
    }
}

