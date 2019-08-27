use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct Align(usize);

#[derive(Debug)]
pub struct AlignErr;

impl fmt::Display for AlignErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("invalid alignment value")
    }
}

impl Align {
    pub fn new(x: usize) -> Result<Self, AlignErr> {
        if x > 0 && x.is_power_of_two() {
            Ok(Self(x - 1))
        } else {
            Err(AlignErr)
        }
    }

    pub fn align_up(&self, x: usize) -> Option<usize> {
        x.checked_add(self.0).map(|x| x & !self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_align() {
        assert!(Align::new(4).is_ok());
        assert!(Align::new(0).is_err());
        assert!(Align::new(3).is_err());
        assert!(Align::new(usize::max_value()).is_err());
    }

    #[test]
    fn align_up() {
        let a1 = Align::new(1).unwrap();

        assert_eq!(a1.align_up(0), Some(0));
        assert_eq!(a1.align_up(1), Some(1));
        assert_eq!(a1.align_up(2), Some(2));
        assert_eq!(a1.align_up(3), Some(3));
        assert_eq!(a1.align_up(4), Some(4));
        assert_eq!(a1.align_up(usize::max_value()), Some(usize::max_value()));

        let a4 = Align::new(4).unwrap();

        assert_eq!(a4.align_up(0), Some(0));
        assert_eq!(a4.align_up(1), Some(4));
        assert_eq!(a4.align_up(2), Some(4));
        assert_eq!(a4.align_up(3), Some(4));
        assert_eq!(a4.align_up(4), Some(4));
        assert_eq!(a4.align_up(5), Some(8));
        assert_eq!(a4.align_up(6), Some(8));
        assert_eq!(a4.align_up(7), Some(8));
        assert_eq!(
            a4.align_up(usize::max_value() - 6),
            Some(usize::max_value() - 3)
        );
        assert_eq!(
            a4.align_up(usize::max_value() - 5),
            Some(usize::max_value() - 3)
        );
        assert_eq!(
            a4.align_up(usize::max_value() - 4),
            Some(usize::max_value() - 3)
        );
        assert_eq!(
            a4.align_up(usize::max_value() - 3),
            Some(usize::max_value() - 3)
        );
        assert_eq!(a4.align_up(usize::max_value() - 2), None);
        assert_eq!(a4.align_up(usize::max_value() - 1), None);
        assert_eq!(a4.align_up(usize::max_value()), None);
    }
}
