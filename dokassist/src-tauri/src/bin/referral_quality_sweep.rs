fn main() {
    if let Err(error) = dokassist_lib::referral_quality_sweep_main() {
        eprintln!("referral-quality-sweep: {error}");
        std::process::exit(2);
    }
}
