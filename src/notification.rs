use std::time::{Duration, Instant};

pub struct Notification {
    hold: Duration,
    fade: Duration,
    peak_alpha: f32,
    shown_at: Option<Instant>,
}

impl Notification {
    pub fn new(hold_seconds: f32, fade_seconds: f32, peak_alpha: f32) -> Self {
        Notification {
            hold: seconds(hold_seconds),
            fade: seconds(fade_seconds),
            peak_alpha: clamp_alpha(peak_alpha),
            shown_at: None,
        }
    }

    pub fn trigger(&mut self, now: Instant) {
        self.shown_at = Some(now);
    }

    pub fn alpha(&self, now: Instant) -> Option<f32> {
        let shown_at = self.shown_at?;
        self.alpha_after(now.saturating_duration_since(shown_at))
    }

    fn alpha_after(&self, elapsed: Duration) -> Option<f32> {
        if elapsed < self.hold {
            return Some(self.peak_alpha);
        }

        let faded = elapsed - self.hold;
        if faded >= self.fade {
            return None;
        }

        let remaining = 1.0 - faded.as_secs_f32() / self.fade.as_secs_f32();
        Some(self.peak_alpha * remaining)
    }
}

fn seconds(value: f32) -> Duration {
    if value.is_finite() && value > 0.0 {
        Duration::from_secs_f32(value)
    } else {
        Duration::ZERO
    }
}

fn clamp_alpha(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use rstest::rstest;

    use super::Notification;

    fn notification() -> Notification {
        Notification::new(3.0, 1.0, 0.75)
    }

    #[rstest]
    #[case::just_shown(0.0, Some(0.75))]
    #[case::during_the_hold(2.9, Some(0.75))]
    #[case::fade_starts(3.0, Some(0.75))]
    #[case::halfway_through_the_fade(3.5, Some(0.375))]
    #[case::almost_gone(3.9, Some(0.075))]
    #[case::fade_finished(4.0, None)]
    #[case::long_after(60.0, None)]
    fn fades_out_after_the_hold(#[case] elapsed: f32, #[case] expected: Option<f32>) {
        let alpha = notification().alpha_after(Duration::from_secs_f32(elapsed));

        match (alpha, expected) {
            (Some(actual), Some(expected)) => assert!(
                (actual - expected).abs() < 1e-5,
                "expected {expected}, got {actual}"
            ),
            (None, None) => {}
            _ => panic!("expected {expected:?}, got {alpha:?}"),
        }
    }

    #[test]
    fn stays_hidden_until_triggered() {
        assert_eq!(notification().alpha(Instant::now()), None);
    }

    #[test]
    fn restarts_the_hold_when_triggered_again() {
        let start = Instant::now();
        let mut notification = notification();

        notification.trigger(start);
        assert_eq!(notification.alpha(start + Duration::from_secs(5)), None);

        notification.trigger(start + Duration::from_secs(5));
        assert_eq!(
            notification.alpha(start + Duration::from_secs(6)),
            Some(0.75)
        );
    }

    #[rstest]
    #[case::negative_hold(-1.0, 1.0, 0.75)]
    #[case::zero_fade(3.0, 0.0, 0.75)]
    #[case::alpha_above_one(3.0, 1.0, 4.0)]
    #[case::not_a_number(f32::NAN, f32::NAN, f32::NAN)]
    fn survives_nonsensical_configuration(
        #[case] hold: f32,
        #[case] fade: f32,
        #[case] alpha: f32,
    ) {
        let start = Instant::now();
        let mut notification = Notification::new(hold, fade, alpha);
        notification.trigger(start);

        let value = notification.alpha(start + Duration::from_millis(1));
        assert!(value.is_none_or(|alpha| (0.0..=1.0).contains(&alpha)));
    }
}
