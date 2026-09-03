macro_rules! retry_policy_builder {
    ($name:ident { $($extra_field:ident : $extra_type:ty = $extra_default:expr),* $(,)? }) => {
        impl Default for $name<Fixed, NoJitter> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name<Fixed, NoJitter> {
            pub fn new() -> Self {
                Self {
                    current: 0,
                    stop_after: None,
                    backoff: Fixed::base(Duration::ZERO),
                    jitterable: NoJitter,
                    max: None,
                    $($extra_field: $extra_default,)*
                }
            }
        }

        impl<Back: Backoff, Jit: Jitter> $name<Back, Jit> {
            /// Sets the maximum number of retries after the initial attempt.
            pub fn stop_after(mut self, retries: u32) -> Self {
                self.stop_after = Some(retries);
                self
            }

            /// Cap the maximum delay between retries.
            pub fn max_delay(mut self, dur: Duration) -> Self {
                self.max = Some(dur);
                self
            }

            /// Wait a fixed amount of time between each retry.
            pub fn fixed(self, dur: Duration) -> $name<Fixed, Jit> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: Fixed::base(dur),
                    jitterable: self.jitterable,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Wait a linearly growing amount of time between each retry `base * attempt`.
            pub fn linear(self, dur: Duration) -> $name<Linear, Jit> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: Linear::base(dur),
                    jitterable: self.jitterable,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Wait an exponentially growing amount of time between each retry `base * 2^attempt`.
            pub fn exponential(self, dur: Duration) -> $name<Exponential, Jit> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: Exponential::base(dur),
                    jitterable: self.jitterable,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Use a custom backoff strategy.
            pub fn backoff<B: Backoff>(self, backoff: B) -> $name<B, Jit> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff,
                    jitterable: self.jitterable,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Use a custom jitter strategy.
            pub fn jitter<J: Jitter>(self, jitter: J) -> $name<Back, J> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: self.backoff,
                    jitterable: jitter,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Random delay between 0 and the backoff value.
            pub fn full_jitter(self) -> $name<Back, crate::jitter::Full> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: self.backoff,
                    jitterable: crate::jitter::Full,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Random delay between backoff/2 and the backoff value.
            pub fn equal_jitter(self) -> $name<Back, crate::jitter::Equal> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: self.backoff,
                    jitterable: crate::jitter::Equal,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Decorrelated jitter: min(max, random(base, previous * 3)).
            pub fn decorrelated_jitter(
                self,
                base: Duration,
            ) -> $name<Back, crate::jitter::Decorrelated> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: self.backoff,
                    jitterable: crate::jitter::Decorrelated::base(base),
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            fn calculate_delay(&mut self, attempt: u32) -> Duration {
                let delay = self.backoff.delay(attempt);
                self.jitterable.jitter(delay, self.max)
            }
        }
    };
}

pub(crate) use retry_policy_builder;
