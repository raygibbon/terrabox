//! Runtime evidence, distinct from internal terminal operation strings.
use crate::terminfo::Capabilities;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Support {
    Supported,
    Unsupported,
    #[default]
    Unknown,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Evidence {
    #[default]
    Unprobed,
    Terminfo,
    EnvironmentHint,
    NoResponse,
    RuntimeReply,
    NativeBackend,
    ApplicationOverride,
    Multiplexer,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapabilityStatus {
    pub support: Support,
    pub evidence: Evidence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    TrueColor,
    Colors256,
    Unicode,
    AlternateScreen,
    Mouse,
    BracketedPaste,
    EnhancedKeyboard,
    KittyGraphics,
    KittyAnimation,
    Sixel,
}
#[derive(Clone, Debug)]
pub struct TerminalCapabilities {
    pub truecolor: CapabilityStatus,
    pub colors256: CapabilityStatus,
    pub unicode: CapabilityStatus,
    pub alternate_screen: CapabilityStatus,
    pub mouse: CapabilityStatus,
    pub bracketed_paste: CapabilityStatus,
    pub enhanced_keyboard: CapabilityStatus,
    pub kitty_graphics: CapabilityStatus,
    pub kitty_animation: CapabilityStatus,
    pub sixel: CapabilityStatus,
    /// Graphics forwarding is intentionally blocked inside detected multiplexers.
    pub multiplexer: bool,
    pub(crate) probed: bool,
    pub(crate) graphics_query: Option<u32>,
}
impl TerminalCapabilities {
    pub(crate) fn new(caps: &Capabilities) -> Self {
        let unknown = CapabilityStatus::default();
        let mut result = Self {
            truecolor: unknown,
            colors256: unknown,
            unicode: unknown,
            alternate_screen: unknown,
            mouse: unknown,
            bracketed_paste: unknown,
            enhanced_keyboard: unknown,
            kitty_graphics: unknown,
            kitty_animation: unknown,
            sixel: unknown,
            multiplexer: std::env::var_os("TMUX").is_some() || std::env::var_os("STY").is_some(),
            probed: false,
            graphics_query: None,
        };
        if let Some(colors) = caps.colors {
            result.colors256 = CapabilityStatus {
                support: if colors >= 256 {
                    Support::Supported
                } else {
                    Support::Unknown
                },
                evidence: Evidence::Terminfo,
            };
            if colors >= 16_777_216 {
                result.truecolor = CapabilityStatus {
                    support: Support::Supported,
                    evidence: Evidence::Terminfo,
                };
            }
        }
        if caps.database {
            result.alternate_screen = CapabilityStatus {
                support: if caps.enter_screen.is_empty() || caps.exit_screen.is_empty() {
                    Support::Unknown
                } else {
                    Support::Supported
                },
                evidence: Evidence::Terminfo,
            };
        }
        if std::env::var("COLORTERM").is_ok_and(|s| s == "truecolor" || s == "24bit")
            && result.truecolor.support == Support::Unknown
        {
            result.truecolor.evidence = Evidence::EnvironmentHint;
        }
        if result.multiplexer {
            result.kitty_graphics.evidence = Evidence::Multiplexer;
        }
        result
    }
    pub(crate) fn finish_probe(&mut self) {
        #[allow(unused_mut)]
        let mut features = vec![
            Capability::TrueColor,
            Capability::Colors256,
            Capability::Mouse,
            Capability::BracketedPaste,
            Capability::EnhancedKeyboard,
            Capability::Sixel,
        ];
        #[cfg(feature = "kitty-graphics")]
        features.push(Capability::KittyGraphics);
        for feature in features {
            let status = self.status_mut(feature);
            if status.support == Support::Unknown
                && status.evidence != Evidence::ApplicationOverride
            {
                status.evidence = if cfg!(windows) {
                    Evidence::NativeBackend
                } else {
                    Evidence::NoResponse
                };
            }
        }
    }
    pub fn get(&self, feature: Capability) -> CapabilityStatus {
        match feature {
            Capability::TrueColor => self.truecolor,
            Capability::Colors256 => self.colors256,
            Capability::Unicode => self.unicode,
            Capability::AlternateScreen => self.alternate_screen,
            Capability::Mouse => self.mouse,
            Capability::BracketedPaste => self.bracketed_paste,
            Capability::EnhancedKeyboard => self.enhanced_keyboard,
            Capability::KittyGraphics => self.kitty_graphics,
            Capability::KittyAnimation => self.kitty_animation,
            Capability::Sixel => self.sixel,
        }
    }
    fn status_mut(&mut self, feature: Capability) -> &mut CapabilityStatus {
        match feature {
            Capability::TrueColor => &mut self.truecolor,
            Capability::Colors256 => &mut self.colors256,
            Capability::Unicode => &mut self.unicode,
            Capability::AlternateScreen => &mut self.alternate_screen,
            Capability::Mouse => &mut self.mouse,
            Capability::BracketedPaste => &mut self.bracketed_paste,
            Capability::EnhancedKeyboard => &mut self.enhanced_keyboard,
            Capability::KittyGraphics => &mut self.kitty_graphics,
            Capability::KittyAnimation => &mut self.kitty_animation,
            Capability::Sixel => &mut self.sixel,
        }
    }
    pub(crate) fn override_status(&mut self, feature: Capability, support: Support) {
        *self.status_mut(feature) = CapabilityStatus {
            support,
            evidence: Evidence::ApplicationOverride,
        };
    }
    fn record(&mut self, feature: Capability, support: Support) {
        let status = self.status_mut(feature);
        if status.evidence != Evidence::ApplicationOverride {
            *status = CapabilityStatus {
                support,
                evidence: Evidence::RuntimeReply,
            };
        }
    }
    /// Returns true only for responses to the bounded queries issued by TerraBox.
    pub(crate) fn observe(&mut self, bytes: &[u8]) -> bool {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return false;
        };
        if let Some(body) = text
            .strip_prefix("\x1bP1+r")
            .and_then(|s| s.strip_suffix("\x1b\\"))
        {
            fn decode(value: &str) -> Option<String> {
                if !value.len().is_multiple_of(2) {
                    return None;
                }
                let bytes: Option<Vec<u8>> = value
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| {
                        std::str::from_utf8(pair)
                            .ok()
                            .and_then(|s| u8::from_str_radix(s, 16).ok())
                    })
                    .collect();
                bytes.and_then(|v| String::from_utf8(v).ok())
            }
            let mut matched = false;
            for item in body.split(';') {
                let (name, value) = item.split_once('=').unwrap_or((item, ""));
                match name.to_ascii_lowercase().as_str() {
                    "524742" => {
                        if value.is_empty()
                            || decode(value).is_some_and(|v| {
                                !v.is_empty()
                                    && v.split(['/', ':'])
                                        .all(|part| part.parse::<u8>().is_ok_and(|n| n > 0))
                            })
                        {
                            self.record(Capability::TrueColor, Support::Supported);
                            matched = true;
                        }
                    }
                    "436f" => {
                        if let Some(count) = decode(value).and_then(|v| v.parse::<u32>().ok()) {
                            self.record(
                                Capability::Colors256,
                                if count >= 256 {
                                    Support::Supported
                                } else {
                                    Support::Unsupported
                                },
                            );
                            matched = true;
                        }
                    }
                    _ => {}
                }
            }
            return matched;
        }
        if let Some(body) = text
            .strip_prefix("\x1bP0+r")
            .and_then(|s| s.strip_suffix("\x1b\\"))
        {
            // Unknown termcap names do not establish feature absence.
            return body.is_empty()
                || matches!(body.to_ascii_lowercase().as_str(), "436f" | "524742");
        }
        if let Some(body) = text
            .strip_prefix("\x1b_G")
            .and_then(|s| s.strip_suffix("\x1b\\"))
        {
            let Some((header, message)) = body.split_once(';') else {
                return false;
            };
            let id = header
                .split(',')
                .find_map(|s| s.strip_prefix("i=")?.parse::<u32>().ok());
            if id.is_some() && id == self.graphics_query {
                if message == "OK" {
                    self.record(Capability::KittyGraphics, Support::Supported);
                } else if message.starts_with("ENOTSUP") || message.starts_with("ENOSYS") {
                    self.record(Capability::KittyGraphics, Support::Unsupported);
                }
                return true;
            }
        }
        if let Some(body) = text.strip_prefix("\x1b[?") {
            if body
                .strip_suffix('u')
                .is_some_and(|s| s.parse::<u32>().is_ok())
            {
                self.record(Capability::EnhancedKeyboard, Support::Supported);
                return true;
            }
            if let Some(modes) = body.strip_suffix("$y").and_then(|s| s.split_once(';')) {
                let feature = match modes.0 {
                    "1006" => Capability::Mouse,
                    "2004" => Capability::BracketedPaste,
                    _ => return false,
                };
                let support = match modes.1 {
                    "1" | "2" | "3" => Support::Supported,
                    "0" | "4" => Support::Unsupported,
                    _ => return false,
                };
                self.record(feature, support);
                return true;
            }
            if let Some(attrs) = body.strip_suffix('c') {
                if attrs.split(';').any(|s| s == "4") {
                    self.record(Capability::Sixel, Support::Supported);
                }
                // Absence of an attribute is not proof that an extension is absent.
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> TerminalCapabilities {
        TerminalCapabilities::new(&Capabilities::fallback("xterm").unwrap())
    }
    #[test]
    fn replies_are_evidence_not_terminal_branding_or_silence() {
        let mut r = report();
        assert_eq!(r.kitty_animation.support, Support::Unknown);
        assert_eq!(r.kitty_graphics.support, Support::Unknown);
        r.graphics_query = Some(123);
        assert!(!r.observe(b"\x1b_Gi=999;OK\x1b\\"));
        assert!(r.observe(b"\x1b_Gi=123;OK\x1b\\"));
        assert_eq!(r.kitty_graphics.support, Support::Supported);
        assert_eq!(r.kitty_animation.support, Support::Unknown);
        assert!(r.observe(b"\x1b[?1006;0$y"));
        assert_eq!(r.mouse.support, Support::Unsupported);
        assert!(r.observe(b"\x1b[?2004;2$y"));
        assert_eq!(r.bracketed_paste.support, Support::Supported);
        assert!(r.observe(b"\x1b[?0u"));
        assert_eq!(r.enhanced_keyboard.support, Support::Supported);
        assert!(r.observe(b"\x1b[?62;4;6c"));
        assert_eq!(r.sixel.support, Support::Supported);
        assert!(!r.observe(b"\x1b[?2004;9$y"));
        r.override_status(Capability::KittyGraphics, Support::Unsupported);
        r.observe(b"\x1b_Gi=123;OK\x1b\\");
        assert_eq!(r.kitty_graphics.evidence, Evidence::ApplicationOverride);
        assert_eq!(r.kitty_graphics.support, Support::Unsupported);
    }
    #[test]
    fn errors_only_confirm_lack_when_the_protocol_explicitly_rejects_support() {
        let mut r = report();
        r.graphics_query = Some(2);
        r.observe(b"\x1b_Gi=2;EINVAL:bad-data\x1b\\");
        assert_eq!(r.kitty_graphics.support, Support::Unknown);
        r.observe(b"\x1b_Gi=2;ENOTSUP:unsupported\x1b\\");
        assert_eq!(r.kitty_graphics.support, Support::Unsupported);
        assert_eq!(r.kitty_animation.support, Support::Unknown);
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    #[test]
    fn xtgettcap_success_and_rejection_have_distinct_meanings() {
        let mut report = TerminalCapabilities::new(&Capabilities::fallback("xterm").unwrap());
        assert!(report.observe(b"\x1bP0+r524742\x1b\\"));
        assert_eq!(report.truecolor.support, Support::Unknown);
        assert!(report.observe(b"\x1bP1+r524742=38;436f=323536\x1b\\"));
        assert_eq!(report.truecolor.support, Support::Supported);
        assert_eq!(report.colors256.support, Support::Supported);
        assert_eq!(report.colors256.evidence, Evidence::RuntimeReply);
        assert!(!report.observe(b"\x1bP1+r436f=ZZ\x1b\\"));
    }
}
