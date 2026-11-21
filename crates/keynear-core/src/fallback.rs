#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FallbackMedia {
    #[default]
    None,
    WifiAwareNan,
    WifiDirectR2,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FallbackStatus {
    #[default]
    Unavailable,
    Available,
    Disallowed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FallbackProbeResult {
    pub media: FallbackMedia,
    pub status: FallbackStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackChoice {
    pub media: FallbackMedia,
    pub status: FallbackStatus,
}

pub fn select_fallback(aware: FallbackProbeResult, direct: FallbackProbeResult) -> FallbackChoice {
    if is_available(aware) {
        return choice(FallbackMedia::WifiAwareNan);
    }
    if is_available(direct) {
        return choice(FallbackMedia::WifiDirectR2);
    }
    choice(FallbackMedia::None)
}

fn is_available(result: FallbackProbeResult) -> bool {
    result.status == FallbackStatus::Available
}

fn choice(media: FallbackMedia) -> FallbackChoice {
    let status = if media == FallbackMedia::None {
        FallbackStatus::Unavailable
    } else {
        FallbackStatus::Available
    };
    FallbackChoice { media, status }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(media: FallbackMedia, status: FallbackStatus) -> FallbackProbeResult {
        FallbackProbeResult { media, status }
    }

    #[test]
    fn aware_is_preferred_when_both_are_available() {
        let aware = probe(FallbackMedia::WifiAwareNan, FallbackStatus::Available);
        let direct = probe(FallbackMedia::WifiDirectR2, FallbackStatus::Available);
        assert_eq!(
            select_fallback(aware, direct).media,
            FallbackMedia::WifiAwareNan
        );
    }

    #[test]
    fn direct_is_used_when_aware_is_disallowed() {
        let aware = probe(FallbackMedia::WifiAwareNan, FallbackStatus::Disallowed);
        let direct = probe(FallbackMedia::WifiDirectR2, FallbackStatus::Available);
        assert_eq!(
            select_fallback(aware, direct).media,
            FallbackMedia::WifiDirectR2
        );
    }

    #[test]
    fn neither_available_returns_unavailable() {
        let aware = probe(FallbackMedia::WifiAwareNan, FallbackStatus::Unavailable);
        let direct = probe(FallbackMedia::WifiDirectR2, FallbackStatus::Disallowed);
        let selected = select_fallback(aware, direct);
        assert_eq!(selected.media, FallbackMedia::None);
        assert_eq!(selected.status, FallbackStatus::Unavailable);
    }
}
