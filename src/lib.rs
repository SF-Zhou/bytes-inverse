#[doc = include_str!("../README.md")]
pub mod core {
    struct Assert<const N: usize>;
    impl<const N: usize> Assert<N> {
        const ASSERT: () = assert!(0 < N && N < 255, "invalid N!");
    }

    /// Represents possible errors that may occur during byte stream mapping operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Error {
        /// The input byte stream is empty
        EmptyBytes,
        /// The input length is invalid (must be a multiple of N+1)
        InvalidLength { len: usize, n: u8 },
        /// A delimiter byte is invalid (must be 0)
        InvalidDelimiter { pos: usize, val: u8 },
        /// A padding byte is invalid (must be 0xFF)
        InvalidPadding { pos: usize, val: u8 },
        /// The ending byte contains invalid padding information
        InvalidEnding { val: u8 },
    }

    /// Maps the input byte stream into a new byte stream.
    ///
    /// This function transforms the input bytes such that for any bytes a < b,
    /// we have map(a) > map(b) in the output stream.
    ///
    /// # Type Parameters
    /// * `N` - The group size, must be between 1 and 255
    ///
    /// # Arguments
    /// * `bytes` - The input byte slice to be mapped
    ///
    /// # Returns
    /// A new vector containing the mapped bytes
    pub fn map<const N: usize>(bytes: &[u8]) -> Vec<u8> {
        _ = Assert::<N>::ASSERT;

        let len = (std::cmp::max(bytes.len(), 1) + N - 1) / N * (N + 1);
        let mut out = Vec::with_capacity(len);
        for (idx, val) in bytes.iter().enumerate() {
            if idx != 0 && idx % N == 0 {
                out.push(0);
            }
            out.push(!val);
        }
        let m = len - 1 - out.len();
        out.resize(len - 1, !0);
        out.push((m + 1) as u8);
        out
    }

    /// Unmaps a previously mapped byte stream back to its original form.
    ///
    /// # Type Parameters
    /// * `N` - The group size, must match the value used in the original mapping
    ///
    /// # Arguments
    /// * `bytes` - The mapped byte slice to be unmapped
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - The original byte stream
    /// * `Err(Error)` - If the input is invalid or corrupted
    ///
    /// # Errors
    /// Returns an error if:
    /// - The input is empty
    /// - The input length is not a multiple of N+1
    /// - Delimiter bytes are not 0
    /// - Padding bytes are not 0xFF
    /// - The ending byte contains invalid padding information
    pub fn unmap<const N: usize>(bytes: &[u8]) -> std::result::Result<Vec<u8>, Error> {
        _ = Assert::<N>::ASSERT;

        if bytes.is_empty() {
            return Err(Error::EmptyBytes);
        }

        let chunks = bytes.len() / (N + 1);
        let mapped_len = chunks * (N + 1);
        if mapped_len != bytes.len() {
            return Err(Error::InvalidLength {
                len: bytes.len(),
                n: N as u8,
            });
        }

        let last = bytes[mapped_len - 1] as usize;
        let padding = if last == 0 || last > N + 1 {
            return Err(Error::InvalidEnding { val: last as u8 });
        } else {
            last - 1
        };

        let unmapped_len = chunks * N - padding;
        let mut out = Vec::with_capacity(unmapped_len);
        for (idx, &val) in bytes.iter().enumerate() {
            if (idx + 1) % (N + 1) == 0 {
                if idx + 1 != mapped_len && val != 0 {
                    return Err(Error::InvalidDelimiter { pos: idx, val });
                }
            } else {
                if out.len() == unmapped_len {
                    if val != 0xff {
                        return Err(Error::InvalidPadding { pos: idx, val });
                    }
                } else {
                    out.push(!val);
                }
            }
        }
        Ok(out)
    }
}

pub use core::Error;

/// Maps a byte stream using the default group size (N=8).
///
/// This is a convenience wrapper around core::map with N=8.
/// See [`core::map`] for detailed documentation.
#[inline(always)]
pub fn map(bytes: &[u8]) -> Vec<u8> {
    core::map::<8>(bytes)
}

/// Unmaps a byte stream using the default group size (N=8).
///
/// This is a convenience wrapper around core::unmap with N=8.
/// See [`core::unmap`] for detailed documentation.
#[inline(always)]
pub fn unmap(bytes: &[u8]) -> std::result::Result<Vec<u8>, Error> {
    core::unmap::<8>(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map() {
        assert!(map(b"") > map(b" "));
        assert!(map(b"a") > map(b"b"));
        assert!(map(b"a") > map(b"aa"));
        assert!(map(b"aa") > map(b"abb"));

        for i in 0..0xff {
            assert!(map(&[]) > map(&vec![i as u8; i + 1]));
            for v in 0..0xff {
                assert!(map(&vec![v; i + 0]) > map(&vec![v; i + 1]));
                assert!(map(&vec![v; i + 1]) > map(&vec![v + 1; i + 1]));
                assert!(map(&vec![v; i + 1]) > map(&vec![v + 1; i + 2]));
                assert!(map(&vec![v; i + 2]) > map(&vec![v + 1; i + 1]));
            }
        }
    }

    #[test]
    fn test_unmap() {
        let bytes_list: &[&[u8]] = &[b"", b"A", b"hello", b"hello world!", b"7268"];
        for bytes in bytes_list {
            assert_eq!(unmap(&map(*bytes)).unwrap(), *bytes);
        }

        for i in 0..0xff {
            for v in 0..=0xff {
                let bytes = vec![v; i];
                let mapped = map(&bytes);
                let unmapped = unmap(&mapped).unwrap();
                assert_eq!(bytes, unmapped);
            }
        }
    }

    #[test]
    fn test_invalid_unmap() {
        assert!(matches!(unmap(b"").unwrap_err(), Error::EmptyBytes));
        assert!(matches!(
            unmap(b"xxxxxxxx").unwrap_err(),
            Error::InvalidLength { len: 8, n: 8 }
        ));
        assert!(matches!(
            unmap(&[1; 18]).unwrap_err(),
            Error::InvalidDelimiter { pos: 8, val: 1 }
        ));
        assert!(matches!(
            unmap(&[2; 9]).unwrap_err(),
            Error::InvalidPadding { pos: 7, val: 2 }
        ));
        assert!(matches!(
            unmap(&[10; 9]).unwrap_err(),
            Error::InvalidEnding { val: 10 }
        ));
    }
}
