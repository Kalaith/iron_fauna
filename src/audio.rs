//! Sound effects — a thin layer over the toolkit `SoundManager`. All SFX are
//! short synthesized WAVs under `assets/sfx/`. Loading failures degrade to
//! silence (never a crash), so the game runs fine with or without an audio
//! device (headless capture, muted browsers, etc.).

use macroquad_toolkit::audio::SoundManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sfx {
    /// UI confirm / menu selection.
    Select,
    /// A weapon or natural attack lands.
    Hit,
    /// A core is cracked — capture, yield, or freed.
    Crack,
    /// A limb is severed.
    Sever,
    /// The rider hops to another mount.
    Hop,
    /// An action was refused (out of vigor, on cooldown, illegal).
    Deny,
    /// A graft is rejected or the creature goes berserk.
    Reject,
}

impl Sfx {
    fn file(self) -> &'static str {
        match self {
            Sfx::Select => "assets/sfx/select.wav",
            Sfx::Hit => "assets/sfx/hit.wav",
            Sfx::Crack => "assets/sfx/crack.wav",
            Sfx::Sever => "assets/sfx/sever.wav",
            Sfx::Hop => "assets/sfx/hop.wav",
            Sfx::Deny => "assets/sfx/deny.wav",
            Sfx::Reject => "assets/sfx/reject.wav",
        }
    }

    const ALL: [Sfx; 7] = [
        Sfx::Select,
        Sfx::Hit,
        Sfx::Crack,
        Sfx::Sever,
        Sfx::Hop,
        Sfx::Deny,
        Sfx::Reject,
    ];
}

/// The game's sound bank. Wraps the toolkit manager and remembers its own
/// per-effect volume trims so a rapid flurry of hits doesn't overpower.
pub struct Audio {
    manager: SoundManager<Sfx>,
}

impl Audio {
    /// Loads every SFX. Missing files are skipped silently.
    pub async fn load() -> Self {
        let mut manager = SoundManager::new();
        manager.sfx_volume = 0.6;
        for sfx in Sfx::ALL {
            let _ = manager.load_sound(sfx, sfx.file()).await;
        }
        Self { manager }
    }

    pub fn play(&self, sfx: Sfx) {
        let vol = match sfx {
            Sfx::Hit => 0.5,
            Sfx::Select => 0.7,
            Sfx::Crack => 0.9,
            _ => 0.8,
        };
        self.manager.play_sfx(sfx, vol);
    }
}
