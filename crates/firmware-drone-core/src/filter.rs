use core::f32::consts::PI;

#[derive(Debug, Default)]
pub struct Filter {
    state: f32,
    alpha: f32,
}

impl Filter {
    pub fn new(cutoff_frequency: f32, sample_period: f32) -> Self {
        let rc = 1.0 / (2.0 * PI * cutoff_frequency);
        let alpha = sample_period / (rc + sample_period);
        Self { state: 0.0, alpha }
    }

    pub fn update(&mut self, input: f32) -> f32 {
        self.state = self.alpha * input + (1.0 - self.alpha) * self.state;
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two f32 values within 1e-4 of each other.
    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn first_sample_from_zero_is_a_partial_step() {
        // From zero state, a step input produces alpha * input, which for any
        // sensible (0, 1) alpha lies strictly between zero and the input.
        let mut f = Filter::new(100.0, 0.01);
        let out = f.update(1.0);
        assert!(out > 0.0 && out < 1.0, "got {out}");
    }

    #[test]
    fn known_numeric_response() {
        // cutoff = 100 Hz, dt = 0.01 s:
        //   RC    = 1 / (2*pi*100)          = 0.00159155
        //   alpha = 0.01 / (RC + 0.01)      = 0.862737
        // First  update(1.0): 0.862737
        // Second update(1.0): 0.862737 + (1-0.862737)*0.862737 = 0.981169
        let mut f = Filter::new(100.0, 0.01);
        assert!(approx(f.update(1.0), 0.862737), "first sample wrong");
        assert!(approx(f.update(1.0), 0.981169), "second sample wrong");
    }

    #[test]
    fn converges_to_constant_input_unity_dc_gain() {
        // A steady input must converge to that same value (DC gain = 1), with no
        // scaling. Feed a constant for long enough and the output should match.
        let mut f = Filter::new(100.0, 0.01);
        let mut out = 0.0;
        for _ in 0..500 {
            out = f.update(5.0);
        }
        assert!(approx(out, 5.0), "did not converge to input, got {out}");
    }

    #[test]
    fn step_response_is_monotonic_and_never_overshoots() {
        // A first-order low-pass has no overshoot: each output rises toward the
        // step value, is non-decreasing, and never exceeds the step.
        let mut f = Filter::new(80.0, 0.01);
        let mut last = 0.0;
        for _ in 0..200 {
            let out = f.update(1.0);
            assert!(out >= last, "not monotonic: {out} < {last}");
            assert!(out <= 1.0, "overshoot: {out} > 1.0");
            last = out;
        }
    }

    #[test]
    fn zero_input_stays_zero() {
        let mut f = Filter::new(100.0, 0.01);
        for _ in 0..10 {
            assert_eq!(f.update(0.0), 0.0);
        }
    }

    #[test]
    fn attenuates_an_alternating_signal() {
        // A signal flipping sign every sample is the Nyquist frequency (50 Hz at
        // 100 Hz sampling). With the cutoff well below that (10 Hz) the filter
        // must attenuate it far below unit amplitude. Steady-state amplitude of
        // an alternating ±1 through a PT1 is alpha/(2-alpha); for a 10 Hz cutoff
        // at dt=0.01 that is ~0.24.
        let mut f = Filter::new(10.0, 0.01);
        let mut sign = 1.0;
        let mut max_mag: f32 = 0.0;
        // Warm up, then measure steady-state amplitude.
        for i in 0..200 {
            let out = f.update(sign);
            if i > 100 {
                max_mag = max_mag.max(out.abs());
            }
            sign = -sign;
        }
        assert!(
            max_mag < 0.3,
            "alternating signal not attenuated: {max_mag}"
        );
    }

    #[test]
    fn higher_cutoff_filters_less() {
        // A higher cutoff means a larger alpha, so after a single step from zero
        // the higher-cutoff filter is closer to the input than the lower one.
        let mut fast = Filter::new(200.0, 0.01);
        let mut slow = Filter::new(20.0, 0.01);
        let fast_out = fast.update(1.0);
        let slow_out = slow.update(1.0);
        assert!(
            fast_out > slow_out,
            "higher cutoff should track faster: fast={fast_out} slow={slow_out}"
        );
    }
}
