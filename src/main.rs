use clap::Parser;
use fm_core::FmRadioSim;
use hound;

#[derive(Parser, Debug)]
#[command(long_about = None)]
struct Args {
    fname: String,
    #[arg(short, long)]
    out: Option<String>,
}

fn main() {
    let args = Args::parse();
    let output_file = if args.out.is_some() {
        args.out.unwrap().clone()
    } else {
        String::from("out.wav")
    };
    let mut reader = hound::WavReader::open(args.fname).unwrap();
    if reader.spec().channels != 2 {
        panic!("Only stereo supported");
    }
    if reader.spec().sample_rate != 44100 {
        panic!("Only 44100 supported");
    }
    const CHUNK_SIZE: usize = 700;

    let samples = reader
        .into_samples::<i16>()
        .map(|s| s.unwrap())
        .collect::<Vec<i16>>();
    let [mut l_samples, mut r_samples] = samples.chunks(2).fold(
        [Vec::<f32>::new(), Vec::<f32>::new()],
        |mut acc, samples| {
            acc[0].push(samples[0] as f32);
            acc[1].push(samples[1] as f32);
            acc
        },
    );
    // let len = l_samples.len();
    while l_samples.len() % CHUNK_SIZE != 0 {
        l_samples.push(0f32);
        r_samples.push(0f32);
    }
    let mut fm_sim = FmRadioSim::from(44100, CHUNK_SIZE, 79_500_000f64);
    fm_sim.init_thread();
    let mut dst_buffer: Vec<i16> = Vec::new();
    let mut l_buffer = vec![0.; CHUNK_SIZE];
    let mut r_buffer = vec![0.; CHUNK_SIZE];
    l_samples
        .chunks(CHUNK_SIZE)
        .zip(r_samples.chunks(CHUNK_SIZE))
        .for_each(|(l, r)| {
            fm_sim.process(l, r, &mut l_buffer, &mut r_buffer);
            l_buffer.iter().zip(r_buffer.iter()).for_each(|(l, r)| {
                dst_buffer.push(*l as i16);
                dst_buffer.push(*r as i16);
            })
        });
    // output
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(output_file, spec).unwrap();
    dst_buffer.into_iter().for_each(|s| {
        writer.write_sample(s).unwrap();
    });
    writer.finalize().unwrap();
}
