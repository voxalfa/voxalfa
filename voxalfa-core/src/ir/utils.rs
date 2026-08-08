use crate::ast::symbols::SymbolId;

#[derive(Debug, Default)]
pub struct UnderlineMarker {
    pub left: bool,
    pub right: bool,
}

#[derive(Debug)]
pub struct UnderlineRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Default)]
pub struct UnderlineBuffer {
    current_sid: Option<SymbolId>,
    current_pos: Option<usize>,
    offset: usize,
    results: Vec<UnderlineRange>,
}

impl UnderlineBuffer {
    pub fn mark(&mut self, sid: SymbolId, position: usize) {
        self.current_sid = Some(sid);

        if let Some(start) = self.current_pos.take() {
            let end = position + self.offset;

            // discard empty targets
            if start != end {
                self.current_sid = None;
                self.results.push(UnderlineRange { start, end });
            }
        } else {
            self.current_pos = Some(position + self.offset);
        }
    }

    pub fn get_trailing(&self) -> Option<SymbolId> {
        self.current_sid
    }

    pub fn add_offset(&mut self, value: usize) {
        self.offset += value;
    }

    pub fn results(&self) -> &[UnderlineRange] {
        &self.results
    }
}

#[derive(Debug)]
pub struct BeatBuffer {
    pub beats: Vec<Vec<usize>>,
}

impl Default for BeatBuffer {
    fn default() -> Self {
        Self {
            beats: vec![vec![]],
        }
    }
}

impl BeatBuffer {
    pub fn append_note(&mut self) {
        self.beats.last_mut().unwrap().push(1);
    }

    pub fn divide(&mut self) {
        self.beats.push(Vec::new());
    }

    pub fn divide_sub(&mut self) {
        if let Some(last) = self.beats.last_mut()
            && last.is_empty()
        {
            last.push(0);
        }
    }

    pub fn get_durations(&self) -> (Vec<usize>, usize) {
        let mut result = Vec::new();

        let denominators = self.get_denominators();
        let divisor = denominators.iter().fold(1, |acc, &d| lcm(acc, d));

        for (i, major_beat) in self.beats.iter().enumerate() {
            for sub_beat in major_beat {
                let duration = divisor / denominators[i];

                if *sub_beat != 0 {
                    result.push(duration);
                } else if let Some(last) = result.last_mut() {
                    *last += duration
                }
            }
        }

        (result, divisor)
    }

    pub fn is_valid(&self) -> bool {
        self.beats.len() <= 2
    }

    fn get_denominators(&self) -> Vec<usize> {
        self.beats
            .iter()
            .map(|sub| self.beats.len() * sub.len())
            .collect()
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b > 0 { gcd(b, a % b) } else { a }
}

fn lcm(a: usize, b: usize) -> usize {
    (a * b) / gcd(a, b)
}

#[cfg(test)]
mod tests {
    use crate::ir::utils::BeatBuffer;

    #[test]
    fn test_beat_distribution() {
        let test_cases: &[(Vec<usize>, usize, fn(&mut BeatBuffer))] = &[
            // d
            (vec![1], 1, |b| b.append_note()),
            // d . r
            (vec![1, 1], 2, |b| {
                b.append_note();
                b.divide();
                b.append_note();
            }),
            // d , r . m , f
            (vec![1, 1, 1, 1], 4, |b| {
                b.append_note();
                b.divide_sub();
                b.append_note();
                b.divide();
                b.append_note();
                b.divide_sub();
                b.append_note();
            }),
            // d . r , m
            (vec![2, 1, 1], 4, |b| {
                b.append_note();
                b.divide();
                b.append_note();
                b.divide_sub();
                b.append_note();
            }),
            // d ., r
            (vec![3, 1], 4, |b| {
                b.append_note();
                b.divide();
                b.divide_sub();
                b.append_note();
            }),
            // d r m
            (vec![1, 1, 1], 3, |b| {
                b.append_note();
                b.append_note();
                b.append_note();
            }),
            // d r m . f s
            (vec![2, 2, 2, 3, 3], 12, |b| {
                b.append_note();
                b.append_note();
                b.append_note();
                b.divide();
                b.append_note();
                b.append_note();
            }),
        ];

        for (duration, total, setup) in test_cases {
            let mut buffer = BeatBuffer::default();

            setup(&mut buffer);

            let result = buffer.get_durations();

            assert!(buffer.is_valid());
            assert_eq!(duration, &result.0);
            assert_eq!(total, &result.1);
        }
    }
}
