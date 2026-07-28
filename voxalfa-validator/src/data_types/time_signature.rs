use crate::ast::solfa::PulseAccent;

#[derive(Debug, Default, Clone)]
pub struct TimeSignature {
    pub top: usize,
    pub bottom: usize,
}

impl TimeSignature {
    pub fn get_accent(&self, position: usize) -> PulseAccent {
        if position == 0 {
            return PulseAccent::Strong;
        }

        let group_size = if self.top.is_multiple_of(3) { 3 } else { 2 };

        if position.is_multiple_of(group_size) {
            PulseAccent::Medium
        } else {
            PulseAccent::Weak
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::solfa::PulseAccent, data_types::time_signature::TimeSignature};

    #[test]
    fn test_time_signature() {
        let test_cases = [
            // | :
            (
                TimeSignature { top: 2, bottom: 4 },
                vec![PulseAccent::Strong, PulseAccent::Weak],
            ),
            // | : :
            (
                TimeSignature { top: 3, bottom: 4 },
                vec![PulseAccent::Strong, PulseAccent::Weak, PulseAccent::Weak],
            ),
            // | : ! :
            (
                TimeSignature { top: 4, bottom: 4 },
                vec![
                    PulseAccent::Strong,
                    PulseAccent::Weak,
                    PulseAccent::Medium,
                    PulseAccent::Weak,
                ],
            ),
            // | : : ! : :
            (
                TimeSignature { top: 6, bottom: 4 },
                vec![
                    PulseAccent::Strong,
                    PulseAccent::Weak,
                    PulseAccent::Weak,
                    PulseAccent::Medium,
                    PulseAccent::Weak,
                    PulseAccent::Weak,
                ],
            ),
        ];

        for (signature, expected) in test_cases {
            let mut pulses = Vec::new();

            for pos in 0..signature.top {
                pulses.push(signature.get_accent(pos));
            }

            assert_eq!(expected, pulses);
        }
    }
}
