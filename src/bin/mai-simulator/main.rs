use maidata::container::parse_maidata_insns;
use maidata::insn::TouchSensor;
use maidata::judge::adapter::convert_note;
use maidata::judge::note::{JudgeNote, Note};
use maidata::judge::simulator::MaiSimulator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let maidata = r#"
(60)
{1}1^2^1^2^1^2^1^2^1^2[1:1],
{1}3,
"#;
    let (notes, state) = parse_maidata_insns(maidata);
    maidata::app::print_state_messages(&state);
    let mut mcx = maidata::materialize::MaterializationContext::with_offset(0.0);
    let mut notes = mcx
        .materialize_insns(&notes)
        .into_iter()
        .map(|note| convert_note((*note).clone()))
        .collect::<Result<Vec<Note>, _>>()?;
    notes.sort_by(|a, b| a.get_start_time().partial_cmp(&b.get_start_time()).unwrap());

    let mut simulator = MaiSimulator::default();
    for note in notes {
        simulator.add_note(note);
    }
    simulator.change_sensor(TouchSensor::new('A', Some(0)).unwrap(), 0.1);
    simulator.change_sensor(TouchSensor::new('A', Some(2)).unwrap(), 3.0);
    simulator.change_sensor(TouchSensor::new('A', Some(2)).unwrap(), 3.01);
    simulator.change_sensor(TouchSensor::new('A', Some(1)).unwrap(), 5.0);
    simulator.change_sensor(TouchSensor::new('A', Some(0)).unwrap(), 5.01);
    simulator.change_sensor(TouchSensor::new('A', Some(1)).unwrap(), 5.01);
    simulator.finish();
    simulator.print_judge_result();

    Ok(())
}
