use rand::Rng;
use rand::rngs::ThreadRng;

pub fn yield_team_name(team_id: u8) -> String {
    match team_id {
        10 => "blue".to_string(),
        11 => "green".to_string(),
        12 => "red".to_string(),
        15 => "purple".to_string(),
        _ => "unknown".to_string()
    }
}

pub fn generate_code(rng: &mut ThreadRng) -> String {
    (0..4).map(|_| rng.gen_range(0..10).to_string()).collect()
}