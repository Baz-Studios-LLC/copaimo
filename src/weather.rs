//! Rain, snow and wind — decided by the ground they fall on.
//!
//! # Chosen from the country, not rolled and filtered
//!
//! The design is specific about this and it is the whole shape of the module: the
//! weather is picked FROM where you are standing rather than rolled for the world
//! and then vetoed where it does not fit. Roll-then-filter reads fine in a summary
//! and is wrong in play — it gives you a world that is "raining" while the desert
//! is merely having the rain hidden, so the sky darkens over the dunes, puddles
//! appear at the border, and the moment anything else asks "is it raining?" it gets
//! the global answer instead of the local one.
//!
//! So: a place has a country, `terrain_core::region` already says which and *how
//! firmly*, and that firmness is what decides how hard it falls. No rain and no
//! snow on the desert, ever. Wind blows anywhere, because wind does.
//!
//! # The border is a slope, not a line
//!
//! Region strength is 1 well inside a country and falls to 0 at its rim — the same
//! number that lets ground cover thin out toward a boundary instead of stopping
//! along one. Reusing it means rain thins as you walk into the dry country and has
//! stopped by the time the sand has, without a line anywhere that it stops at.
//!
//! # It comes off the clock, like everything else here
//!
//! The sun reads the machine's hour and the seasons read its date, so the weather
//! reads its hour too — a smooth curve over hashed values, one sample every few
//! hours. That buys three things: it is the same weather for everybody, it can be
//! asked about any hour at all without simulating the ones in between, and it is
//! testable, which a stream of `rand` calls is not.

use bevy::prelude::*;

use crate::config::{COLD_SNOWLINE, SNOWLINE};
use crate::season::{Season, TheYear};
use crate::world::StreamAnchor;
use crate::world::terrain::TerrainSource;

/// How many hours one draw of weather lasts before it becomes the next one.
///
/// Five, so a wet morning can clear by the afternoon but the sky is not flickering
/// between states while you cross a field.
const HOURS_PER_DRAW: f64 = 5.0;

/// Where the sky starts thickening, well below where it starts raining.
///
/// Clouds gather BEFORE the rain and stay after it: a sky that is clear one moment
/// and shedding water the next has no weather in it, only a switch. This is the
/// bottom of the build-up and `FALLS_ABOVE` is where the first drop lands, so there
/// is a stretch of every wet spell that is overcast and dry.
const CLOUDS_GATHER_ABOVE: f32 = 0.25;

/// Above this much wet, something is falling.
///
/// Not zero, or it would drizzle permanently: most draws sit in the middle of the
/// range, so a threshold well up it is what makes clear weather the common case.
const FALLS_ABOVE: f32 = 0.55;

/// Above this much cold, what falls is snow rather than rain.
const FREEZES_ABOVE: f32 = 0.5;

/// What is coming down.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Falling {
    #[default]
    Nothing,
    Rain,
    Snow,
}

impl Falling {
    pub fn name(self) -> &'static str {
        match self {
            Falling::Nothing => "clear",
            Falling::Rain => "rain",
            Falling::Snow => "snow",
        }
    }
}

/// The weather where the player is standing.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TheWeather {
    pub falling: Falling,
    /// How hard, 0 to 1. Already thinned by the country underfoot.
    pub fall: f32,
    /// How hard the wind blows, 0 to 1.
    pub wind: f32,
    /// The way the wind blows, as a flat unit vector.
    pub wind_way: Vec2,
    /// How closed the sky is, 0 clear to 1 solid. Leads the rain and outlasts it.
    pub overcast: f32,
    /// Hours added on top of the clock, for looking at a storm on purpose.
    pub nudge: f64,
    /// False once somebody has scrubbed it, until they ask for it back.
    pub follows_clock: bool,
}

impl Default for TheWeather {
    fn default() -> Self {
        Self {
            falling: Falling::Nothing,
            fall: 0.0,
            wind: 0.2,
            wind_way: Vec2::X,
            overcast: 0.0,
            nudge: 0.0,
            follows_clock: true,
        }
    }
}

impl TheWeather {
    /// How the HUD says it.
    pub fn spoken(&self) -> String {
        let held = if self.follows_clock { "" } else { " (held)" };
        match self.falling {
            Falling::Nothing => format!("clear, wind {:.2}{held}", self.wind),
            other => format!("{} {:.2}, wind {:.2}{held}", other.name(), self.fall, self.wind),
        }
    }
}

/// A smooth value in 0..1 from a stream of hashed samples.
///
/// One sample every `HOURS_PER_DRAW`, eased between, so the weather arrives and
/// leaves rather than switching. `stream` separates the wet curve from the windy
/// one so a gale is not tied to a downpour.
fn drawn(hours: f64, stream: u64) -> f32 {
    let place = hours / HOURS_PER_DRAW;
    let step = place.floor();
    let into = (place - step) as f32;

    let hash = |n: i64| -> f32 {
        // A plain integer hash, seeded with the WORLD.
        //
        // Deterministic across machines and runs, which a `DefaultHasher` is
        // explicitly not promised to be - two players in the same world are under
        // the same sky, and a test can ask about any hour without simulating the
        // ones before it. `WORLD_SEED` is what keeps that from meaning "every world
        // gets the same year of weather": change the seed and the whole sequence is
        // different, while any one world's own weather stays its own.
        let mut x = (n as u64)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ stream.wrapping_mul(0xD6E8_FEB8_6659_FD93)
            ^ (crate::config::WORLD_SEED as u64).wrapping_mul(0xA24B_AED4_963E_E407);
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 11) as f32 / (1u64 << 53) as f32
    };

    let a = hash(step as i64);
    let b = hash(step as i64 + 1);
    let ease = into * into * (3.0 - 2.0 * into);
    a + (b - a) * ease
}

/// How dry the ground is here, 0 to 1, where 1 is the heart of the desert.
///
/// Continuous across the border by construction: the desert's own strength falls to
/// nought at its rim, and everywhere beyond it is another country and reads nought
/// too — so there is no step anywhere for rain to stop at.
pub fn how_dry(country: terrain_core::region::Country, share: f32) -> f32 {
    match country {
        terrain_core::region::Country::Desert => share.clamp(0.0, 1.0),
        _ => 0.0,
    }
}

/// How cold it is here, 0 to 1, from the country, the season and the altitude.
///
/// Three reasons for snow rather than one, because all three are true: the snow
/// country is cold all year, winter is cold everywhere, and high ground is cold
/// whatever the month. The strongest wins rather than the sum — a mountain in
/// winter is not twice as frozen as a mountain, it is just frozen.
pub fn how_cold(
    country: terrain_core::region::Country,
    share: f32,
    season: Season,
    height: f32,
) -> f32 {
    let by_country = match country {
        terrain_core::region::Country::Snow => share.clamp(0.0, 1.0),
        _ => 0.0,
    };
    let by_season = match season {
        Season::Winter => 0.75,
        Season::Autumn => 0.25,
        Season::Spring => 0.20,
        Season::Summer => 0.0,
    };
    // Between the cold country's snowline and the ordinary one's, so a peak is
    // frozen in July and the valley under it is not.
    let by_height = ((height - COLD_SNOWLINE) / (SNOWLINE - COLD_SNOWLINE)).clamp(0.0, 1.0);
    by_country.max(by_season).max(by_height)
}

/// The weather for one place and one hour.
///
/// Everything the module decides, in one pure function, so a test can ask it about
/// any hour, any country and any season without a world or a clock.
pub fn weather_at(
    hours: f64,
    country: terrain_core::region::Country,
    share: f32,
    season: Season,
    height: f32,
) -> Sky {
    let wet = drawn(hours, 1);
    let windy = drawn(hours, 2);
    // A slow turn rather than a fresh direction each draw: wind backs and veers,
    // it does not jump round the compass.
    let angle = drawn(hours, 3) * std::f32::consts::TAU;

    let dry = how_dry(country, share);
    let cold = how_cold(country, share, season, height);

    // The country decides how much of the draw ever reaches the ground.
    let reaching = (wet - FALLS_ABOVE).max(0.0) / (1.0 - FALLS_ABOVE);
    let fall = reaching * (1.0 - dry);

    let falling = if fall <= 0.0 {
        Falling::Nothing
    } else if cold >= FREEZES_ABOVE {
        Falling::Snow
    } else {
        Falling::Rain
    };

    // Wind blows anywhere, desert included, and gets a floor so the world is never
    // completely still. A storm carries more of it.
    let wind = (0.12 + windy * 0.75 + fall * 0.25).clamp(0.0, 1.0);

    // The sky closes over from well below the rain line, and the dry country keeps
    // its clear sky: clouds that gather over a desert and never shed anything read
    // as a bug in the weather rather than as weather.
    let overcast = crate::util::smoothstep(CLOUDS_GATHER_ABOVE, 0.95, wet) * (1.0 - dry);

    Sky {
        falling,
        fall: fall.clamp(0.0, 1.0),
        wind,
        wind_way: Vec2::from_angle(angle),
        overcast,
    }
}

/// What the sky is doing at one place and hour.
#[derive(Clone, Copy, Debug)]
pub struct Sky {
    pub falling: Falling,
    pub fall: f32,
    pub wind: f32,
    pub wind_way: Vec2,
    pub overcast: f32,
}

/// Hours since a fixed point, from the machine's clock.
///
/// The same clock the sun and the seasons read. Counted from the epoch rather than
/// from midnight so the weather does not repeat itself every day.
fn hours_now() -> f64 {
    use chrono::Timelike;
    let now = chrono::Local::now();
    let day = chrono::Datelike::num_days_from_ce(&now.date_naive()) as f64;
    day * 24.0 + now.hour() as f64 + now.minute() as f64 / 60.0 + now.second() as f64 / 3600.0
}

/// Reads the sky over wherever the player is standing.
fn read_the_weather(
    keys: Res<ButtonInput<KeyCode>>,
    year: Res<TheYear>,
    terrain: Res<TerrainSource>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    mut weather: ResMut<TheWeather>,
) {
    if keys.just_pressed(KeyCode::F1) {
        weather.follows_clock = false;
        weather.nudge -= HOURS_PER_DRAW;
    }
    if keys.just_pressed(KeyCode::F2) {
        weather.follows_clock = false;
        weather.nudge += HOURS_PER_DRAW;
    }
    if keys.just_pressed(KeyCode::F4) {
        weather.follows_clock = true;
        weather.nudge = 0.0;
    }

    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let at = anchor.translation();
    let (country, share) = terrain.region(at.x, at.z);
    let height = terrain.height(at.x, at.z);
    let hours = hours_now() + weather.nudge;

    let sky = weather_at(hours, country, share, year.season, height);
    weather.falling = sky.falling;
    weather.fall = sky.fall;
    weather.wind = sky.wind;
    weather.wind_way = sky.wind_way;
    weather.overcast = sky.overcast;
}

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TheWeather>()
            .add_systems(Update, read_the_weather);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terrain_core::region::Country;

    /// Every hour of a long stretch, so nothing here passes by sampling.
    const A_LONG_WHILE: i32 = 24 * 400;

#[test]
    fn the_sky_closes_over_before_it_rains_and_never_over_the_desert() {
        let mut gathered_dry = 0;
        for hour in 0..A_LONG_WHILE {
            let hours = hour as f64;
            let green = weather_at(hours, Country::Ordinary, 1.0, Season::Summer, 20.0);
            let desert = weather_at(hours, Country::Desert, 1.0, Season::Summer, 20.0);

            // It cannot rain out of a clear sky.
            if green.fall > 0.0 {
                assert!(
                    green.overcast > 0.0,
                    "hour {hour}: {} falling out of a sky {:.2} closed",
                    green.fall,
                    green.overcast
                );
            }
            // Overcast and dry is a real state and has to happen, or the clouds
            // are just a wetness meter with a different name.
            if green.overcast > 0.3 && green.fall == 0.0 {
                gathered_dry += 1;
            }
            assert_eq!(
                desert.overcast, 0.0,
                "hour {hour}: the desert sky closed to {:.2}",
                desert.overcast
            );
        }
        assert!(
            gathered_dry > 0,
            "the sky never once gathered without raining"
        );
    }

    #[test]
    fn the_weather_does_not_repeat_itself() {
        // Random, not a cycle. Compared against ITSELF at every offset up to a
        // fortnight: a sequence with a period in it matches at that period.
        let year: Vec<f32> = (0..A_LONG_WHILE)
            .map(|h| weather_at(h as f64, Country::Ordinary, 1.0, Season::Summer, 20.0).fall)
            .collect();
        for period in 1..(24 * 14) {
            let pairs = year.len() - period;
            let same = (0..pairs)
                .filter(|&i| (year[i] - year[i + period]).abs() < 1e-4)
                .count();
            assert!(
                same * 4 < pairs * 3,
                "the weather repeats every {period} hours: {same} of {pairs} hours match"
            );
        }
        // And it is varied rather than sitting near one value all year.
        let mean = year.iter().sum::<f32>() / year.len() as f32;
        let spread =
            (year.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / year.len() as f32).sqrt();
        assert!(spread > 0.1, "a whole year of weather varies by only {spread:.3}");
    }

    #[test]
    fn nothing_ever_falls_on_the_desert() {
        // The rule the design states outright, asked the only way worth asking it:
        // over every hour of more than a year, in every season, at every altitude
        // the desert has - and in the heart of it AND on its rim.
        for hour in 0..A_LONG_WHILE {
            let hours = hour as f64;
            for season in Season::ALL {
                for share in [1.0, 0.75, 0.5, 0.25, 0.01] {
                    for height in [0.0, 60.0, 140.0, 300.0] {
                        let it = weather_at(hours, Country::Desert, share, season, height);
                        let (falling, fall) = (it.falling, it.fall);
                        if share >= 0.999 {
                            assert_eq!(
                                falling,
                                Falling::Nothing,
                                "hour {hour} in {} put {} on the deep desert",
                                season.name(),
                                falling.name()
                            );
                            assert_eq!(fall, 0.0, "the deep desert got {fall} of fall");
                        }
                        // And wherever it is partly desert, it never falls HARDER
                        // than the same hour would on ordinary ground.
                        let plain = weather_at(hours, Country::Ordinary, 1.0, season, height).fall;
                        assert!(
                            fall <= plain + 1e-6,
                            "hour {hour}: dry ground got {fall} against green ground's {plain}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn rain_thins_towards_the_dry_country_instead_of_stopping_at_a_line() {
        // Walk the border: strength 1 in the heart of the desert down to 0 at its
        // rim, where the next country begins at full strength. The fall has to rise
        // smoothly across that and meet the green world's value when it arrives.
        let wet = (0..A_LONG_WHILE)
            .map(|h| h as f64)
            .find(|&h| weather_at(h, Country::Ordinary, 1.0, Season::Summer, 20.0).fall > 0.3)
            .expect("some hour in a year is wet");

        let mut last = 0.0;
        let mut worst_step: f32 = 0.0;
        for step in 0..=100 {
            let share = 1.0 - step as f32 / 100.0;
            let fall = weather_at(wet, Country::Desert, share, Season::Summer, 20.0).fall;
            if step > 0 {
                worst_step = worst_step.max((fall - last).abs());
            }
            last = fall;
        }
        let green = weather_at(wet, Country::Ordinary, 1.0, Season::Summer, 20.0).fall;
        assert!(
            (last - green).abs() < 1e-6,
            "at the desert's rim it falls {last} and just outside it falls {green} — \
             that is a line for the rain to stop at"
        );
        assert!(
            worst_step < 0.05,
            "the fall jumps {worst_step:.3} in a hundredth of the border"
        );
    }

    #[test]
    fn the_wind_blows_everywhere_including_the_desert() {
        let mut still = 0;
        for hour in 0..A_LONG_WHILE {
            let it = weather_at(hour as f64, Country::Desert, 1.0, Season::Summer, 20.0);
            let (wind, way) = (it.wind, it.wind_way);
            assert!(wind > 0.0, "hour {hour} in the desert had no wind at all");
            assert!(
                (way.length() - 1.0).abs() < 1e-3,
                "the wind blows {} of a direction",
                way.length()
            );
            if wind < 0.2 {
                still += 1;
            }
        }
        // And it is not a constant: it has calm hours as well as loud ones.
        assert!(
            still > 0,
            "the desert wind never dropped below 0.2 in {A_LONG_WHILE} hours"
        );
    }

    #[test]
    fn the_cold_country_gets_snow_and_the_summer_lowlands_get_rain() {
        let mut snow_up_north = 0;
        let mut rain_down_south = 0;
        for hour in 0..A_LONG_WHILE {
            let hours = hour as f64;
            let north = weather_at(hours, Country::Snow, 1.0, Season::Summer, 60.0).falling;
            let south = weather_at(hours, Country::Ordinary, 1.0, Season::Summer, 20.0).falling;
            assert_ne!(north, Falling::Rain, "it rained on the snow country");
            assert_ne!(
                south,
                Falling::Snow,
                "it snowed on the lowlands in high summer"
            );
            snow_up_north += (north == Falling::Snow) as i32;
            rain_down_south += (south == Falling::Rain) as i32;
        }
        assert!(snow_up_north > 0, "it never snowed on the snow country");
        assert!(rain_down_south > 0, "it never rained on the green world");
    }

    #[test]
    fn winter_turns_the_rain_to_snow_and_a_peak_is_frozen_in_july() {
        let wet = (0..A_LONG_WHILE)
            .map(|h| h as f64)
            .find(|&h| weather_at(h, Country::Ordinary, 1.0, Season::Summer, 20.0).fall > 0.3)
            .expect("some hour in a year is wet");

        let summer_valley = weather_at(wet, Country::Ordinary, 1.0, Season::Summer, 20.0).falling;
        let winter_valley = weather_at(wet, Country::Ordinary, 1.0, Season::Winter, 20.0).falling;
        let summer_peak =
            weather_at(wet, Country::Ordinary, 1.0, Season::Summer, SNOWLINE + 30.0).falling;

        assert_eq!(summer_valley, Falling::Rain);
        assert_eq!(winter_valley, Falling::Snow, "a winter valley got rain");
        assert_eq!(summer_peak, Falling::Snow, "a summer peak got rain");
    }

    #[test]
    fn the_weather_arrives_and_leaves_rather_than_switching() {
        // Sampled every six minutes across a fortnight: nothing may jump.
        let mut worst: f32 = 0.0;
        let mut last = weather_at(0.0, Country::Ordinary, 1.0, Season::Summer, 20.0).fall;
        for step in 1..(24 * 14 * 10) {
            let hours = step as f64 / 10.0;
            let now = weather_at(hours, Country::Ordinary, 1.0, Season::Summer, 20.0).fall;
            worst = worst.max((now - last).abs());
            last = now;
        }
        // 0.08 in six minutes, which is a shower arriving over about an hour and a
        // quarter. It measures 0.053, so about two hours.
        //
        // The first bar was 0.02 and it was wrong, not the code: 0.02 in six
        // minutes is a change that takes FIVE HOURS to complete, which is not
        // weather turning, it is weather creeping. What this test is for is
        // catching a sky that SWITCHES - a step, an instant downpour - and an hour
        // is comfortably on the right side of that.
        assert!(
            worst < 0.08,
            "the fall moves {worst:.3} in six minutes — the sky is switching, not turning"
        );
    }

    #[test]
    fn clear_weather_is_the_common_case() {
        let wet = (0..A_LONG_WHILE)
            .filter(|&h| {
                weather_at(h as f64, Country::Ordinary, 1.0, Season::Summer, 20.0).falling
                    != Falling::Nothing
            })
            .count();
        let share = wet as f32 / A_LONG_WHILE as f32;
        assert!(
            (0.05..0.45).contains(&share),
            "it is falling {:.0}% of the time in the green world",
            share * 100.0
        );
    }
}
