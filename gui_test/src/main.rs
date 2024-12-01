use iced::time;

use iced::{
    executor,
    widget::{Column, Container, Text},
    Alignment, Application, Command, Element, Length, Settings, Subscription,
    Theme,
};
use plotters::{coord::Shift, prelude::*};
use plotters_iced::{Chart, ChartWidget};
use spectrum_analyzer::scaling::scale_to_zero_to_one;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
const TITLE_FONT_SIZE: u16 = 22;
use fm_core::FmRadioSim;
// PARAMETERS
const BUFFER_SIZE: usize = 730;
const AUDIO_SAMPLE_RATE: usize = 44_100;

const SIGNAL_FREQ: f64 = 440f64;
// const CARRIER_FREQ: f64 = 84_700_000f64;
const CARRIER_FREQ: f64 = 79_500_000f64;
const A: f64 = 0.5;
const RENDER_MAX: usize = 100;
// is modulate audio sig
const DISABLE_AUDIO_INPUT: bool = false;
const FIXED_RENDERING_DURATION: u64 = 100;  // 1ms1
const ENABLE_FIXED_TIME_RENDER:bool = true;
const FRAME_TIME:f64 = BUFFER_SIZE as f64 / AUDIO_SAMPLE_RATE as f64;

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
    None,
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
                if self.chart.is_draw() {
                    self.chart.next();
                }
            }
            Message::None => {}
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
        time::every(time::Duration::from_millis(
          if ENABLE_FIXED_TIME_RENDER {FIXED_RENDERING_DURATION} else {FRAME_TIME.ceil() as u64})).map(|_| Message::Tick)
    }
}
// use std::collections::VecDeque;


struct MyChart {
    t: f64,
    render_times: usize,
    disable_audio_in: bool,
    // signals
    input_signal: [Vec<f32>; 2],
    output_signal_l: Vec<f32>,
    output_signal_r: Vec<f32>,
    continue_flag: bool,
    // transmission_line: transmission_line::TransmissionLine,
    fm_radio_sim: FmRadioSim,
}
impl MyChart {
    pub fn new() -> Self {
        println!(
            "Signal time per frame: {}ms",
            (BUFFER_SIZE as f64) / AUDIO_SAMPLE_RATE as f64 * 1000f64
        );
        let mut fm_radio_sim =
            FmRadioSim::from(AUDIO_SAMPLE_RATE, BUFFER_SIZE, CARRIER_FREQ);
        fm_radio_sim.init_thread();
        Self {
            render_times: 0,
            t: 0.0,
            disable_audio_in: DISABLE_AUDIO_INPUT,
            // Buffer
            input_signal: [vec![0.; BUFFER_SIZE], vec![0.; BUFFER_SIZE]],

            output_signal_l: vec![0.; BUFFER_SIZE],
            output_signal_r: vec![0.; BUFFER_SIZE],
            fm_radio_sim,
            continue_flag: true,
        }
    }

    fn view(&self) -> Element<Message> {
        let chart = ChartWidget::new(self)
            .width(Length::Fill)
            .height(Length::Fill);

        chart.into()
    }
    fn is_draw(&self) -> bool {
        self.continue_flag
    }
    fn next(&mut self) {
        use std::time::Instant;
        if self.continue_flag && self.render_times < RENDER_MAX {
            // 信号の作成
            for i in 0..self.input_signal[0].len() {
                self.input_signal[0][i] = (self.t as f32
                    * 2.
                    * std::f32::consts::PI
                    * SIGNAL_FREQ as f32)
                    .sin()
                    * A as f32;
                self.input_signal[1][i] = (self.t as f32
                    * 2.
                    * std::f32::consts::PI
                    * SIGNAL_FREQ as f32
                    * 2.)
                    .sin()
                    * A as f32;
                self.t += 1f64 / AUDIO_SAMPLE_RATE as f64;
            }
            // println!("start processing");
            // up-sample
            let timer = Instant::now();
            self.fm_radio_sim.process(
                &self.input_signal[0],
                &self.input_signal[1],
                &mut self.output_signal_l,
                &mut self.output_signal_r,
            );
            let end_time = timer.elapsed();
            // println!("================================");
            println!("Elapsed Time: {:?}", end_time);
            // println!("  - Up-Sample: {:?}", lap0);
            // println!("  - Composite: {:?}", lap1 - lap0);
            // println!("  - Up-Sample: {:?}", lap2 - lap1);
            // println!("  - Modulate: {:?}", lap3 - lap2);
            // println!("  - Cvt-IntermediateFreq: {:?}", lap4 - lap3);
            // println!("  - Filtering: {:?}", lap5 - lap4);
            // println!("  - De-Modulate: {:?}", lap6 - lap5);
            // println!("  - Down-Sample: {:?}", lap7 - lap6);
            // println!("  - Restore: {:?}", lap8 - lap7);
            // println!("  - Down-Sample: {:?}", end_time - lap8);
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

        let labels: [&str; 12] = [
            "L In",
            "R In",
            "Composite",
            "Freq Modulated",
            "Intermediate",
            "Freq Demodulated",
            "L Out",
            "R Out",
            "Intermediate Spectrum",
            "Demodulate Spectrum",
            "",
            "",
        ];
        for (i, area) in children.iter().enumerate() {
            let builder = ChartBuilder::on(area);
            match i {
                0 => draw_chart(
                    builder,
                    labels[i],
                    &self.input_signal[0]
                        .iter()
                        .map(|x| *x as f64)
                        .collect::<Vec<_>>(),
                    AUDIO_SAMPLE_RATE,
                ),
                1 => draw_chart(
                    builder,
                    labels[i],
                    &self.input_signal[1]
                        .iter()
                        .map(|x| *x as f64)
                        .collect::<Vec<_>>(),
                    AUDIO_SAMPLE_RATE,
                ),

                // 3 => draw_chart(
                //     builder,
                //     labels[i],
                //     &self.modulated_signal,
                //     FM_MODULATION_SAMPLE_RATE,
                // ),
                4 => draw_chart(
                    builder,
                    labels[i],
                    self.fm_radio_sim.get_intermediate(),
                    FmRadioSim::FM_MODULATION_SAMPLE_RATE/ FmRadioSim::RATIO_FS_INTER_FS,
                ),
                // 5 => draw_chart(
                //     builder,
                //     labels[i],
                //     &self.demodulated_signal,
                //     FM_MODULATION_SAMPLE_RATE / RATIO_FS_INTER_FS,
                // ),
                6 => draw_chart(
                    builder,
                    labels[i],
                    &self
                        .output_signal_l
                        .iter()
                        .map(|x| *x as f64)
                        .collect::<Vec<_>>(),
                    AUDIO_SAMPLE_RATE,
                ),
                7 => draw_chart(
                    builder,
                    labels[i],
                    &self
                        .output_signal_r
                        .iter()
                        .map(|x| *x as f64)
                        .collect::<Vec<_>>(),
                    AUDIO_SAMPLE_RATE,
                ),
                // 8 => draw_spectrum(
                //   builder,
                //   labels[i],
                //   &self.intermediate,
                //   FM_MODULATION_SAMPLE_RATE >> 2,
                //   FrequencyLimit::All,
                // ),
                // 9 => draw_spectrum(
                //   builder,
                //   labels[i],
                //   &self.demodulated_signal,
                //   FM_MODULATION_SAMPLE_RATE >> 2,
                //   FrequencyLimit::All,
                // ),
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

fn draw_spectrum<DB: DrawingBackend>(
    mut chart: ChartBuilder<DB>,
    label: &str,
    data: &[f64],
    sample_rate: usize,
    limit: FrequencyLimit,
) {
    let n = {
        let mut n = (data.len()) as u64;
        let mut x = 64;
        while n & 0x8000_0000_0000_0000 == 0 {
            n <<= 1;
            x -= 1;
        }
        1 << (x - 1)
    };
    let spectrum = samples_fft_to_spectrum(
        data.iter()
            .take(n.min(2048))
            .map(|x| *x as f32)
            .collect::<Vec<f32>>()
            .as_slice(),
        sample_rate as u32,
        limit,
        Some(&scale_to_zero_to_one),
    )
    .unwrap();
    let mut chart = chart
        .margin(10)
        .caption(
            format!("{} ({}Hz)", label, spectrum.max().0),
            ("sans-serif", 22),
        )
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..sample_rate as f32 / 2f32, 0f32..1f32)
        .unwrap();

    chart
        .configure_mesh()
        .x_labels(5)
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
            spectrum.data().iter().map(|(f, x)| (f.val(), x.val())),
            // (-50..=50)
            //     .map(|x| x as f32 / 50.0)
            //     .map(|x| (x, x.powf(power as f32))),
            &RED,
        ))
        .unwrap();
}
#[inline]
fn get_buffer_size(s1: usize, s2: usize, base_size: usize) -> usize {
    (s2 as f64 / s1 as f64 * base_size as f64 + 0.5).floor() as usize
}
