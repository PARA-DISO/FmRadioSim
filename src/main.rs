use iced::time;
use iced::{
    executor,
    widget::{Column, Container, Text},
    window, Alignment, Application, Command, Degrees, Element, Font, Length, Point, Rectangle,
    Renderer, Settings, Subscription, Theme, Vector,
};
use plotters::{coord::Shift, prelude::*};
use plotters_backend::DrawingBackend;
use plotters_iced::{plotters_backend, Chart, ChartWidget, DrawingArea};
use spectrum_analyzer::scaling::{divide_by_N_sqrt, scale_20_times_log10, scale_to_zero_to_one};
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
const TITLE_FONT_SIZE: u16 = 22;
mod filter;
mod fm_modulator;
use filter::Lpf;

mod composite;
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
            .push(Text::new("Iced test chart").size(TITLE_FONT_SIZE))
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
        time::every(time::Duration::from_millis(50)).map(|_| Message::Tick)
    }
}
// use std::collections::VecDeque;
const SIZE: usize = 512;
const AUDIO_SAMPLE_RATE: usize = 44100;
const SAMPLE_RATE: usize = 500_000 * 4;
const SIGNAL_FREQ: f64 = 440_f64;
const CARRIER_FREQ: f64 = 500_000f64;

use fm_modulator::{FmDeModulator, FmModulator};

use composite::{CompositeSignal, RestoredSignal};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
struct MyChart {
    t: f64,
    lr: [Vec<f32>; 2],
    sig: Vec<f32>,
    carrier: Vec<f32>,
    modulator: FmModulator,
    demodulator: FmDeModulator,
    composite: CompositeSignal,
    restor: RestoredSignal,
    up_sampler: FastFixedIn<f32>,
    continue_flag: bool,
}
impl MyChart {
    pub fn new() -> Self {
      // let composite = CompositeSignal::new(AUDIO_SAMPLE_RATE as f32, SIZE);
      // let up_sampler =  FastFixedIn::new(
      //           SAMPLE_RATE as f64 / composite.sample_rate() as f64,
      //           SAMPLE_RATE as f64 / composite.sample_rate() as f64,
      //           PolynomialDegree::Linear,
      //           SIZE,
      //           1,
      //       )
      //       .unwrap();
        let composite = CompositeSignal::new(SAMPLE_RATE as f32, SIZE);
      let up_sampler =  FastFixedIn::new(
                SAMPLE_RATE as f64 / AUDIO_SAMPLE_RATE as f64,
                SAMPLE_RATE as f64 / AUDIO_SAMPLE_RATE as f64,
                PolynomialDegree::Linear,
                SIZE,
                1,
            )
            .unwrap();
      let restor = RestoredSignal::new(SAMPLE_RATE as f32);
        Self {
            t: 0.0,
            sig: vec![0f32; SIZE],
            lr: [vec![0.; SIZE], vec![0.; SIZE]],
            carrier: vec![0f32; SIZE],
            modulator: FmModulator::from(CARRIER_FREQ, SAMPLE_RATE as f64),
            demodulator: FmDeModulator::from(CARRIER_FREQ, SAMPLE_RATE as f64),
            composite,
            restor,
            up_sampler,
            
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
        if self.continue_flag {
            // 信号の作成
            for i in 0..SIZE {
                // self.sig[i] = ((self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ).sin() as f32 + (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ * 2.).sin() as f32);
                // self.sig[i] = (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ).sin() as f32;
                // self.carrier[i] = ((self.t * 2f64 * std::f64::consts::PI * CARRIER_FREQ).cos() as f32);

                self.lr[0][i] = (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ).sin() as f32;
                self.lr[1][i] =
                    (self.t * 2f64 * std::f64::consts::PI * SIGNAL_FREQ * 2.).sin() as f32;
                self.t += 1f64 / AUDIO_SAMPLE_RATE as f64;
            }
            // upsample
            // dbg!(self.up_sampler.output_frames_next());
            
             let l = self
                .up_sampler
                .process(&[&self.lr[0]], None)
                .unwrap();
            let r = self
              .up_sampler
              .process(&[&self.lr[1]], None)
              .unwrap();
            self.composite.process(&l[0],&r[0]);
           
            // 変調
            let modulated = self.modulator.modulate(self.composite.get_buffer());
            // 復調
            self.demodulator.demodulate(modulated);
            // コンポジット
            self.restor.process(self.demodulator.get_buffer());
            
            // dbg!(&self.sig);
            // unreachable!();
            // self.continue_flag = false;
        }
    }
}
use rustfft::num_complex::ComplexFloat;
impl Chart<Message> for MyChart {
    type State = ();
    // leave it empty
    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, _builder: ChartBuilder<DB>) {}

    fn draw_chart<DB: DrawingBackend>(&self, _state: &Self::State, root: DrawingArea<DB, Shift>) {
        // let  serufu = unsafe {(self as *const Self).cast_mut().as_mut().unwrap()};
        let children = root.split_evenly((3, 4));
        // let labels: [&str; 12] = [
        //     "input",
        //     "input spectrum",
        //     "carrier",
        //     "carrier spectrum",
        //     "modulated",
        //     "modulated spectrum",
        //     "lpf",
        //     "",
        //     "demodulate",
        //     "demodulated spectrum",
        //     "",
        //     "",
        // ];
        let labels: [&str; 12] = [
            "L",
            "R",
            "L+R",
            "L-R",
            "Composite",
            "Decomposite L",
            "Decomposite R",
            "FM Modulated",
            "FM Demodulated",
            "",
            "demodulate",
            "demodulated spectrum",
        ];
        let modurated_buffer = self.modulator.get_buffer();
        let demodulate = self.demodulator.get_buffer();
        for (i, area) in children.iter().enumerate() {
            let builder = ChartBuilder::on(area);
            match i {
                // 0 => draw_chart(builder, labels[0], &self.sig, AUDIO_SAMPLE_RATE),
                // 1 => draw_spectrum(builder, labels[1], &self.sig, AUDIO_SAMPLE_RATE),
                // 2 => {
                //     if !modurated_buffer.is_empty() {
                //         draw_chart(builder, labels[4], modurated_buffer, SAMPLE_RATE)
                //     }
                // }
                // 3 => draw_chart(builder, labels[8], demodulate, SAMPLE_RATE),
                0 => draw_chart(builder, labels[0], &self.lr[0], AUDIO_SAMPLE_RATE),
                1 => draw_chart(builder, labels[1], &self.lr[1], AUDIO_SAMPLE_RATE),
                2 => draw_chart(
                    builder,
                    labels[2],
                    &(self.lr[0])
                        .iter()
                        .zip((self.lr[1]).iter())
                        .map(|(&l, &r)| l + r)
                        .collect::<Vec<f32>>(),
                    AUDIO_SAMPLE_RATE,
                ),
                3 => draw_chart(
                    builder,
                    labels[3],
                    &(self.lr[0])
                        .iter()
                        .zip((self.lr[1]).iter())
                        .map(|(&l, &r)| l - r)
                        .collect::<Vec<f32>>(),
                    AUDIO_SAMPLE_RATE,
                ),
                4 => draw_chart(
                    builder,
                    labels[2],
                    self.composite.get_buffer(),
                    self.composite.sample_rate() as usize,
                ),
                5 => draw_chart(
                   builder,
                    labels[7],
                    self.modulator.get_buffer(),
                    SAMPLE_RATE
                ),
                6 => draw_chart(
                   builder,
                    labels[8],
                    self.demodulator.get_buffer(),
                    SAMPLE_RATE
                ),
                7 => draw_chart(
                    builder,
                    labels[5],
                    &self.restor.get_buffer()[0],
                    SAMPLE_RATE,
                ),
                8 => draw_chart(
                    builder,
                    labels[6],
                    &self.restor.get_buffer()[1],
                    SAMPLE_RATE,
                ),
                // 7 => {
                //     if !self.restor.get_buffer()[1].is_empty() {
                //         draw_spectrum(
                //             builder,
                //             "spectrum A",
                //             &self.restor.get_buffer()[0],
                //             AUDIO_SAMPLE_RATE,
                //         );
                //     }
                // }
                // 8 => {
                //     if !self.restor.get_buffer()[1].is_empty() {
                //         draw_spectrum(
                //             builder,
                //             "spectrum B",
                //             &self.restor.get_buffer()[1],
                //             AUDIO_SAMPLE_RATE,
                //         );
                //     }
                // }
                _ => {}
            }
        }
    }
}
fn draw_chart<DB: DrawingBackend>(
    mut chart: ChartBuilder<DB>,
    label: &str,
    data: &[f32],
    sample_rate: usize,
) {
    let mut chart = chart
        .margin(30)
        .caption(label, ("sans-serif", 22))
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..data.len() as f32 / sample_rate as f32, -3f32..3f32)
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
                .map(|(i, x)| (i as f32 / sample_rate as f32, *x)),
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
    data: &[f32],
    sample_rate: usize,
) {
    let spectrum = samples_fft_to_spectrum(
        data,
        sample_rate as u32,
        FrequencyLimit::All,
        Some(&scale_to_zero_to_one),
    )
    .unwrap();
    let mut chart = chart
        .margin(30)
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
