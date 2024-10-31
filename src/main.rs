use iced::time;

use iced::{
    executor,
    widget::{Column, Container, Text},
    Alignment, Application, Command, Element, Length, Settings, Subscription,
    Theme,
};
use plotters::{coord::Shift, prelude::*};
use plotters_iced::{Chart, ChartWidget};
// use spectrum_analyzer::scaling::scale_to_zero_to_one;
// use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
const TITLE_FONT_SIZE: u16 = 22;
mod fm_modulation;
use fm_modulation::*;
// mod transmission_line;
use std::ffi::c_void;
#[link(name = "freq_modulation")]
extern "C" {
    pub fn fm_modulate(
        output_signal: *mut f64,
        input_signal: *const f64,
        prev_sig: *mut f64,
        sum: *mut f64,
        sample_periodic: f64,
        angle: *mut f64,
        modulate_index: f64,
        fc: f64,
        buf_len: u64,
    );
    fn convert_intermediate_freq(
      output_signal: *mut f64,
      input_signal: *const f64,
      sample_period: f64,
      fc: f64, fi: f64,
      info: *mut crate::fm_modulator::CnvFiInfos, buf_len: usize);
    fn fm_demodulate(
        output_signal: *mut f64,
        input_signal: *const f64,
        sample_period: f64,
        carrier_freq: f64,
        info: *mut crate::fm_modulator::DemodulationInfo,
        buf_len: u64,
    );
    fn upsample(dst: *mut f64, input: *const f64, info: *mut ResamplerInfo);
    fn downsample(dst: *mut f64, input: *const f64, info: *mut ResamplerInfo);
}
#[repr(C)]
pub struct ResamplerInfo {
    prev: f64,
    multiplier: usize,
    input_len: usize,
}

impl ResamplerInfo {
    pub fn new_upsample_info(
        src_fs: usize,
        dst_fs: usize,
        input_size: usize,
    ) -> Self {
        Self {
            prev: 0.0,
            multiplier: dst_fs / src_fs,
            input_len: input_size,
        }
    }
    pub fn new_downsample_info(
        src_fs: usize,
        dst_fs: usize,
        input_size: usize,
    ) -> Self {
        Self {
            prev: 0.0,
            multiplier: src_fs / dst_fs,
            input_len: input_size,
        }
    }
}
// #[link(name="resampler")]
// extern "C" {

// }
fn main() {
    State::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
    .unwrap();
}

#[derive(Debug)]
enum Message {
    Tick,
}

struct State {
    chart: MyChart,
}

impl Application for State {
    type Message = self::Message;
    type Executor = executor::Default;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                chart: MyChart::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Split Chart Example".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Tick => {
                self.chart.next();
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content = Column::new()
            .spacing(20)
            .align_items(Alignment::Start)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(Text::new("FM Stereo").size(TITLE_FONT_SIZE))
            .push(self.chart.view());

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(5)
            .center_x()
            .center_y()
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        // window::frames().map(|_| Message::Tick)
        time::every(time::Duration::from_millis(1000)).map(|_| Message::Tick)
    }
}
// use std::collections::VecDeque;
const BUFFER_SIZE: usize = 256;
const TEST_BUFFER_SIZE: usize = 256;
const AUDIO_SAMPLE_RATE: usize = 44_100;
const COMPOSITE_SAMPLE_RATE: usize = 132_300;
// const FM_MODULATION_SAMPLE_RATE: usize = 176_400_000;
// const FM_MODULATION_SAMPLE_RATE: usize = 220500000;
// const FM_MODULATION_SAMPLE_RATE: usize = (79_500_000 * 3);

// const FM_MODULATION_SAMPLE_RATE: usize = 882_000_000;
const SIGNAL_FREQ: f64 = 440f64;
// const FM_MODULATION_SAMPLE_RATE: usize = 352_800_000;
const FM_MODULATION_SAMPLE_RATE: usize = 264_600_000;
const CARRIER_FREQ: f64 =      79_500_000f64;
const INTERMEDIATE_FREQ: f64 = 10_700_000f64;
const SIGNAL_MAX_FREQ: f64 = 106_000f64;
// const CARRIER_FREQ: f64 = 79_500_0f64;
// const CUT_OFF: f64 = 200_000.;
const CUT_OFF: f64 = 0.;
const NOISE: f32 = -70.;
const A: f64 = 0.5;
const RENDER_MAX: usize = 10;
use fm_modulator::*;

use composite::{CompositeSignal, RestoredSignal};
use libsoxr::{
    datatype::Datatype,
    spec::{QualityFlags, QualityRecipe, QualitySpec},
    Soxr,
};
use rubato::{FastFixedIn, FastFixedOut, PolynomialDegree, Resampler};
struct MyChart {
    t: f64,
    render_times: usize,
    fm_sample_rate: usize,
    // convertor/modulator
    composite: CompositeSignal,
    restore: RestoredSignal,
    modulator: FmModulator,
    demodulator: FmDeModulator,
    cvt_intermediate: CvtIntermediateFreq,
    // signals
    input_signal: [Vec<f64>; 2],
    up_sampled_input: [Vec<f64>; 2],
    composite_signal: Vec<f64>,
    restored_signal: [Vec<f64>; 2],
    output_signal: [Vec<f64>; 2],
    resampled_composite: Vec<f64>,
    modulated_signal: Vec<f64>,
    intermediate: Vec<f64>,
    demodulated_signal: Vec<f64>,
    resampled_demodulate: Vec<f64>,
    // Re-Sampler
    // up_sampler_to100k: FastFixedIn<f64>,
    // down_sampler_to_output: FastFixedOut<f64>,
    // up_sample_to176m: FastFixedIn<f64>,
    // down_sample_to_100k: FastFixedOut<f64>,
    up_sampler_to100k: [Soxr; 2],
    down_sampler_to_output: [Soxr; 2],
    // up_sample_to176m: Soxr,
    up_sample_to176m: ResamplerInfo,
    down_sample_to_100k: ResamplerInfo,
    // down_sample_to_100k: Soxr,
    continue_flag: bool,
    // transmission_line: transmission_line::TransmissionLine,
}
impl MyChart {
    pub fn new() -> Self {
        println!(
            "fs/fc: {}",
            FM_MODULATION_SAMPLE_RATE as f32 / CARRIER_FREQ as f32
        );
        let fm_sample_rate = {
            let tmp = (FM_MODULATION_SAMPLE_RATE as f64
                / COMPOSITE_SAMPLE_RATE as f64)
                .ceil() as usize;
            (((tmp & 0xffff_ffff_ffff_fff8)
                + if tmp & 0b111 != 0 { 4 } else { 0 })
                * COMPOSITE_SAMPLE_RATE)
        };
        println!("fm sample: {fm_sample_rate}");
        println!("fi / fcompo: {}", (fm_sample_rate >> 1) as f64 / COMPOSITE_SAMPLE_RATE as f64);
        let up_sampler_to100k = [
            Soxr::create(
                AUDIO_SAMPLE_RATE as f64,
                COMPOSITE_SAMPLE_RATE as f64,
                1,
                Some(&libsoxr::IOSpec::new(
                    Datatype::Float64I,
                    Datatype::Float64I,
                )),
                Some(&QualitySpec::new(
                    &QualityRecipe::Quick,
                    QualityFlags::ROLLOFF_NONE,
                )),
                None,
            )
            .unwrap(),
            Soxr::create(
                AUDIO_SAMPLE_RATE as f64,
                COMPOSITE_SAMPLE_RATE as f64,
                1,
                Some(&libsoxr::IOSpec::new(
                    Datatype::Float64I,
                    Datatype::Float64I,
                )),
                Some(&QualitySpec::new(
                    &QualityRecipe::Quick,
                    QualityFlags::ROLLOFF_NONE,
                )),
                None,
            )
            .unwrap(),
        ];
        let down_sampler_to_output = [
            Soxr::create(
                COMPOSITE_SAMPLE_RATE as f64,
                AUDIO_SAMPLE_RATE as f64,
                1,
                Some(&libsoxr::IOSpec::new(
                    Datatype::Float64I,
                    Datatype::Float64I,
                )),
                Some(&QualitySpec::new(
                    &QualityRecipe::Quick,
                    QualityFlags::ROLLOFF_NONE,
                )),
                None,
            )
            .unwrap(),
            Soxr::create(
                COMPOSITE_SAMPLE_RATE as f64,
                AUDIO_SAMPLE_RATE as f64,
                1,
                Some(&libsoxr::IOSpec::new(
                    Datatype::Float64I,
                    Datatype::Float64I,
                )),
                Some(&QualitySpec::new(
                    &QualityRecipe::Quick,
                    QualityFlags::ROLLOFF_NONE,
                )),
                None,
            )
            .unwrap(),
        ];
        let composite_buffer_size = get_buffer_size(
            AUDIO_SAMPLE_RATE,
            COMPOSITE_SAMPLE_RATE,
            BUFFER_SIZE,
        );
        let modulated_buffer_size = get_buffer_size(
            COMPOSITE_SAMPLE_RATE,
            fm_sample_rate,
            composite_buffer_size,
        );
        let up_sample_to176m = ResamplerInfo::new_upsample_info(
            COMPOSITE_SAMPLE_RATE,
            fm_sample_rate,
            composite_buffer_size,
        );
        let down_sample_to_100k = ResamplerInfo::new_downsample_info(
            fm_sample_rate >> 1,
            COMPOSITE_SAMPLE_RATE,
            modulated_buffer_size >> 1,
        );
        
        let composite = CompositeSignal::new(COMPOSITE_SAMPLE_RATE as f64);

        let restore = RestoredSignal::new(COMPOSITE_SAMPLE_RATE as f64);

        println!(
            "Signal time per frame: {}ms",
            (BUFFER_SIZE as f64) / AUDIO_SAMPLE_RATE as f64 * 1000f64
        );
        Self {
            render_times: 0,
            t: 0.0,
            fm_sample_rate,
            // Modulator
            composite,
            restore,
            modulator: FmModulator::from(
                CARRIER_FREQ,
                FM_MODULATION_SAMPLE_RATE as f64,
            ),
            demodulator: FmDeModulator::from(
                // CARRIER_FREQ ,
                INTERMEDIATE_FREQ,
                (FM_MODULATION_SAMPLE_RATE >> 1) as f64,
                SIGNAL_MAX_FREQ,
            ),
            cvt_intermediate: CvtIntermediateFreq::new(
              FM_MODULATION_SAMPLE_RATE as f64,
              CARRIER_FREQ,
              INTERMEDIATE_FREQ
            ),
            // Buffer
            input_signal: [vec![0.; BUFFER_SIZE], vec![0.; BUFFER_SIZE]],
            up_sampled_input: [
                vec![0.; composite_buffer_size],
                vec![0.; composite_buffer_size],
            ],
            composite_signal: vec![0.; composite_buffer_size],
            resampled_composite: vec![0.; modulated_buffer_size],
            modulated_signal: vec![0.; modulated_buffer_size],
            intermediate: vec![0.; modulated_buffer_size >> 1],
            demodulated_signal: vec![0.; modulated_buffer_size >> 1],
            resampled_demodulate: vec![0.; composite_buffer_size],
            restored_signal: [
                vec![0.; composite_buffer_size],
                vec![0.; composite_buffer_size],
            ],
            output_signal: [vec![0.; BUFFER_SIZE], vec![0.; BUFFER_SIZE]],
            // Re-Sampler
            up_sampler_to100k,
            down_sampler_to_output,
            up_sample_to176m,
            down_sample_to_100k,
            continue_flag: true,
        }
    }

    fn view(&self) -> Element<Message> {
        let chart = ChartWidget::new(self)
            .width(Length::Fill)
            .height(Length::Fill);

        chart.into()
    }
    fn next(&mut self) {
        use std::time::Instant;
        if self.continue_flag && self.render_times < RENDER_MAX {
            // 信号の作成
            for i in 0..self.input_signal[0].len() {
                self.input_signal[0][i] =
                    (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ).sin()
                        * A;
                self.input_signal[1][i] =
                    (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ * 2.)
                        .sin()
                        * A;
                self.t += 1f64 / AUDIO_SAMPLE_RATE as f64;
            }

            // up-sample
            let timer = Instant::now();
            // let resample_info = self
            //     .up_sampler_to100k
            //     .process_into_buffer(
            //         &self.input_signal,
            //         &mut self.up_sampled_input,
            //         None,
            //     )
            //     .unwrap();

            let left_upsample_info = self.up_sampler_to100k[0]
                .process::<f64, f64>(
                    Some(&self.input_signal[0]),
                    &mut self.up_sampled_input[0], // &mut self.composite_signal
                );
            let right_upsample_info = self.up_sampler_to100k[1]
                .process::<f64, f64>(
                    Some(&self.input_signal[1]),
                    &mut self.up_sampled_input[1],
                );
            let lap0 = timer.elapsed();
            println!(
                "left: {:?}, right: {:?}",
                left_upsample_info, right_upsample_info
            );
            // composite
            self.composite.process_to_buffer(
                &self.up_sampled_input[0],
                &self.up_sampled_input[1],
                &mut self.composite_signal,
            );
            let lap1 = timer.elapsed();
            // up-sample to MHz Order
            // let vhf_write_size = self.up_sample_to176m.process_into_buffer(
            //     &[&self.composite_signal],
            //     &mut [&mut self.resampled_composite],
            //     None,
            // );
            // let _ = self.up_sample_to176m.process::<f64, f64>(
            //     Some(&self.composite_signal),
            //     &mut self.resampled_composite, // &mut self.demodulated_signal
            // );
            unsafe {
                upsample(
                    self.resampled_composite.as_mut_ptr(),
                    self.composite_signal.as_ptr(),
                    &raw mut self.up_sample_to176m,
                );
            }
            let lap2 = timer.elapsed();
            // Modulate
            self.modulator.process_to_buffer(
                &self.resampled_composite,
                &mut self.modulated_signal,
            );
            let lap3 = timer.elapsed();
            self.cvt_intermediate.process(&self.modulated_signal, &mut self.intermediate);
            let lap4 = timer.elapsed();
            // // de-modulate
            self.demodulator.process_to_buffer(
                &self.intermediate,
                &mut self.demodulated_signal,
            );
            let lap5 = timer.elapsed();
            // down-sample to 100kHz Order
            // let _ = self.down_sample_to_100k.process_into_buffer(
            //     &[&self.demodulated_signal],
            //     &mut [&mut self.resampled_demodulate],
            //     None,
            // );
            unsafe {
                downsample(
                    self.resampled_demodulate.as_mut_ptr(),
                    self.demodulated_signal.as_ptr(),
                    &raw mut self.down_sample_to_100k,
                );
            }
            // let _ = self.down_sample_to_100k.process::<f64,f64>(
            //   Some(&self.demodulated_signal),
            //   &mut self.resampled_demodulate
            // );
            let lap6 = timer.elapsed();
            // restore
            let l_out = unsafe {
                std::slice::from_raw_parts_mut(
                    self.restored_signal
                        .get_unchecked(0)
                        .as_slice()
                        .as_ptr()
                        .cast_mut(),
                    self.restored_signal.get_unchecked(0).len(),
                )
            };
            let r_out = unsafe {
                std::slice::from_raw_parts_mut(
                    self.restored_signal
                        .get_unchecked(1)
                        .as_slice()
                        .as_ptr()
                        .cast_mut(),
                    self.restored_signal.get_unchecked(1).len(),
                )
            };
            self.restore.process_to_buffer(
                &self.resampled_demodulate,
                l_out,
                r_out,
            );
            let lap7 = timer.elapsed();
            // let down_sampled_size = self
            //     .down_sampler_to_output
            //     .process_into_buffer(
            //         &self.restored_signal,
            //         &mut self.output_signal,
            //         None,
            //     )
            //     .unwrap();
            let _ = self.down_sampler_to_output[0].process(
                Some(&self.restored_signal[0]),
                //  Some(&self.resampled_demodulate),
                &mut self.output_signal[0],
            );
            let _ = self.down_sampler_to_output[1].process(
                Some(&self.restored_signal[1]),
                &mut self.output_signal[1],
            );
            let end_time = timer.elapsed();
            println!("================================");
            println!("Elapsed Time: {:?}", end_time);
            println!("  - Up-Sample: {:?}", lap0);
            println!("  - Composite: {:?}", lap1 - lap0);
            println!("  - Up-Sample: {:?}", lap2 - lap1);
            println!("  - Modulate: {:?}", lap3 - lap2);
            println!("  - Cvt-IntermediateFreq: {:?}", lap4 - lap3);
            println!("  - De-Modulate: {:?}", lap5 - lap4);
            println!("  - Down-Sample: {:?}", lap6 - lap5);
            println!("  - Restore: {:?}", lap7 - lap6);
            println!("  - Down-Sample: {:?}", end_time - lap7);
            // println!("Buffer Size: {}/ Resampled Size: {:?}", self.modulated_signal.len(),vhf_write_size);
            // println!("Finally Buffer Size: {}/ Resampled Size: {:?}", self.output_signal[0].len(),down_sampled_size);
            self.render_times += 1;
        }
    }
}

impl Chart<Message> for MyChart {
    type State = ();
    // leave it empty
    fn build_chart<DB: DrawingBackend>(
        &self,
        _state: &Self::State,
        _builder: ChartBuilder<DB>,
    ) {
    }

    fn draw_chart<DB: DrawingBackend>(
        &self,
        _state: &Self::State,
        root: DrawingArea<DB, Shift>,
    ) {
        let children = root.split_evenly((3, 4));

        let labels: [&str; 8] = [
            "L In",
            "R In",
            "Composite",
            "FM Modulated",
            "Intermediate",
            "FM Demodulated",
            "L Out",
            "R Out",
        ];
        for (i, area) in children.iter().enumerate() {
            let builder = ChartBuilder::on(area);
            match i {
                0 => draw_chart(
                    builder,
                    labels[i],
                    &self.input_signal[0],
                    AUDIO_SAMPLE_RATE,
                ),
                1 => draw_chart(
                    builder,
                    labels[i],
                    &self.input_signal[1],
                    AUDIO_SAMPLE_RATE,
                ),
                2 => draw_chart(
                    builder,
                    labels[i],
                    &self.composite_signal,
                    COMPOSITE_SAMPLE_RATE,
                ),
                // 2 => draw_chart(
                //       builder,
                //       labels[i],
                //       &self.resampled_composite,
                //       FM_MODULATION_SAMPLE_RATE,
                //   ),
                3 => draw_chart(
                    builder,
                    labels[i],
                    &self.modulated_signal,
                    FM_MODULATION_SAMPLE_RATE,
                ),
                4 => draw_chart(
                  builder,
                  labels[i],
                  &self.intermediate,
                  FM_MODULATION_SAMPLE_RATE >> 1,
                ),
                5 => draw_chart(
                    builder,
                    labels[i],
                    &self.resampled_demodulate,
                    COMPOSITE_SAMPLE_RATE,
                ),
                6 => draw_chart(
                    builder,
                    labels[i],
                    &self.output_signal[0],
                    AUDIO_SAMPLE_RATE,
                ),
                7 => draw_chart(
                    builder,
                    labels[i],
                    &self.output_signal[1],
                    AUDIO_SAMPLE_RATE,
                ),
                _ => {}
            }
        }
    }
}
fn draw_chart<DB: DrawingBackend>(
    mut chart: ChartBuilder<DB>,
    label: &str,
    data: &[f64],
    sample_rate: usize,
) {
    let mut chart = chart
        .margin(10)
        .caption(label, ("sans-serif", 22))
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            0f64..data.len() as f64 / sample_rate as f64,
            -1f64..1f64,
        )
        .unwrap();

    chart
        .configure_mesh()
        .x_labels(3)
        .y_labels(3)
        // .y_label_style(
        //     ("sans-serif", 15)
        //         .into_font()
        //         .color(&plotters::style::colors::BLACK.mix(0.8))
        //         .transform(FontTransform::RotateAngle(30.0)),
        // )
        .draw()
        .unwrap();

    chart
        .draw_series(LineSeries::new(
            data.iter()
                // .take(SIZE)
                .enumerate()
                .map(|(i, x)| (i as f64 / sample_rate as f64, *x)),
            // (-50..=50)
            //     .map(|x| x as f32 / 50.0)
            //     .map(|x| (x, x.powf(power as f32))),
            &RED,
        ))
        .unwrap();
}

// fn draw_spectrum<DB: DrawingBackend>(
//     mut chart: ChartBuilder<DB>,
//     label: &str,
//     data: &[f32],
//     sample_rate: usize,
//     limit: FrequencyLimit,
// ) {
//     let spectrum = samples_fft_to_spectrum(
//         data,
//         sample_rate as u32,
//         limit,
//         Some(&scale_to_zero_to_one),
//     )
//     .unwrap();
//     let mut chart = chart
//         .margin(10)
//         .caption(
//             format!("{} ({}Hz)", label, spectrum.max().0),
//             ("sans-serif", 22),
//         )
//         .x_label_area_size(30)
//         .y_label_area_size(30)
//         .build_cartesian_2d(0f32..sample_rate as f32 / 2f32, 0f32..1f32)
//         .unwrap();

//     chart
//         .configure_mesh()
//         .x_labels(5)
//         .y_labels(3)
//         // .y_label_style(
//         //     ("sans-serif", 15)
//         //         .into_font()
//         //         .color(&plotters::style::colors::BLACK.mix(0.8))
//         //         .transform(FontTransform::RotateAngle(30.0)),
//         // )
//         .draw()
//         .unwrap();
//     chart
//         .draw_series(LineSeries::new(
//             spectrum.data().iter().map(|(f, x)| (f.val(), x.val())),
//             // (-50..=50)
//             //     .map(|x| x as f32 / 50.0)
//             //     .map(|x| (x, x.powf(power as f32))),
//             &RED,
//         ))
//         .unwrap();
// }
#[inline]
fn get_buffer_size(s1: usize, s2: usize, base_size: usize) -> usize {
    (s2 as f64 / s1 as f64 * base_size as f64 + 0.5).floor() as usize
}
