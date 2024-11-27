mod modulation_modules;
use libsoxr::{
    datatype::Datatype,
    spec::{QualityFlags, QualityRecipe, QualitySpec},
    Soxr,
};
use modulation_modules::*;
mod resampler;
use resampler::*;
#[link(name = "freq_modulation")]
extern "C" {
    fn fm_modulate(
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
        fc: f64,
        fi: f64,
        info: *mut modulator::CnvFiInfos,
        buf_len: usize,
    );
    fn fm_demodulate(
        output_signal: *mut f64,
        input_signal: *const f64,
        sample_period: f64,
        carrier_freq: f64,
        info: *mut modulator::DemodulationInfo,
        buf_len: u64,
    );
    fn upsample(dst: *mut f64, input: *const f64, info: *mut ResamplerInfo);
    fn downsample(dst: *mut f64, input: *const f64, info: *mut ResamplerInfo);
    fn filtering(
        output_signal: *mut f64,
        input_signal: *const f64,
        filter_coeff: *mut modulator::BandPassFilter,
        buf_len: u64,
    );
}
pub struct FmRadioSim {
    // basic parameters
    audio_sample_rate: usize,
    buffer_size: usize,
    // Simulation Modules
    composite: composite::CompositeSignal,
    restore: composite::RestoreSignal,
    modulator: modulator::Modulator,
    demodulator: modulator::DeModulator,
    freq_converter: modulator::CvtIntermediateFreq,
    bandpass_filter: modulator::BandPassFilter,
    // resampler
    upsampler: [Soxr; 2],
    downsampler: [Soxr; 2],
    upsampler_for_radio_waves: ResamplerInfo,
    downsampler_for_radio_waves: ResamplerInfo,
    // internal buffer
    // interleave/de-interleave
    tmp_buffer: [Vec<f64>; 2],      // audio sample rate
    audio_in_buffer: [Vec<f64>; 2], // 125kHz
    composite_signal: Vec<f64>,     // 125kHz
    up_sampled_signal: Vec<f64>,    // 192MHz
    modulate_signal: Vec<f64>,      // 192MHz
    intermediate_signal: Vec<f64>,  // 192MHz (inner 384MHz)
    intermediate_signal_out: Vec<f64>, // 48MHz
    demodulate_signal: Vec<f64>,    // 48MHz
    post_down_sample: Vec<f64>,     // 125kHz
    restored_signal_l: Vec<f64>,
    restored_signal_r: Vec<f64>, // 125kHz
}
impl FmRadioSim {
    // define constants
    const COMPOSITE_SAMPLE_RATE: usize = 125_000;
    const FM_MODULATION_SAMPLE_RATE: usize = 192_000_000;
    const INTERMEDIATE_FREQ: f64 = 10_700_000f64; // JISC6421:1994
    const SIGNAL_MAX_FREQ: f64 = 53_000. * 2.; // x2 Composite freq max
    const RATIO_FS_INTER_FS: usize = 4;
    //
    pub fn from(
        audio_fs: usize,
        buffer_size: usize,
        carrier_freq: f64,
    ) -> Self {
        // calc basic params
        let fm_sample_rate = get_8x_sample_rate(
            Self::FM_MODULATION_SAMPLE_RATE,
            Self::COMPOSITE_SAMPLE_RATE,
        );
        let intermediate_fs = fm_sample_rate / Self::RATIO_FS_INTER_FS;
        // generate soxr
        let upsampler = [
            generate_resampler(
                audio_fs as f64,
                Self::COMPOSITE_SAMPLE_RATE as f64,
            )
            .unwrap(),
            generate_resampler(
                audio_fs as f64,
                Self::COMPOSITE_SAMPLE_RATE as f64,
            )
            .unwrap(),
        ];
        let downsampler = [
            generate_resampler(
                Self::COMPOSITE_SAMPLE_RATE as f64,
                audio_fs as f64,
            )
            .unwrap(),
            generate_resampler(
                Self::COMPOSITE_SAMPLE_RATE as f64,
                audio_fs as f64,
            )
            .unwrap(),
        ];
        // calculate buffer size
        let composite_buffer_size =
            get_buffer_size(audio_fs, Self::COMPOSITE_SAMPLE_RATE, buffer_size);
        let modulated_buffer_size = get_buffer_size(
            Self::COMPOSITE_SAMPLE_RATE,
            fm_sample_rate,
            composite_buffer_size,
        );
        let intermediate_buffer_size =
            modulated_buffer_size / Self::RATIO_FS_INTER_FS;
        // MHz order resampler init
        let upsampler_for_radio_waves = ResamplerInfo::new_upsample_info(
            Self::COMPOSITE_SAMPLE_RATE,
            fm_sample_rate,
            composite_buffer_size,
        );
        let downsampler_for_radio_waves = ResamplerInfo::new_downsample_info(
            intermediate_fs,
            Self::COMPOSITE_SAMPLE_RATE,
            intermediate_buffer_size,
        );
        Self {
            audio_sample_rate: audio_fs,
            buffer_size,
            //
            composite: composite::CompositeSignal::new(
                Self::COMPOSITE_SAMPLE_RATE as f64,
            ),
            restore: composite::RestoreSignal::new(
                Self::COMPOSITE_SAMPLE_RATE as f64,
            ),
            modulator: modulator::Modulator::from(
                carrier_freq,
                fm_sample_rate as f64,
            ),
            freq_converter: modulator::CvtIntermediateFreq::new(
                fm_sample_rate as f64,
                carrier_freq,
                Self::INTERMEDIATE_FREQ,
            ),
            bandpass_filter: modulator::BandPassFilter::new(
                fm_sample_rate as f64,
                Self::INTERMEDIATE_FREQ,
            ),
            demodulator: modulator::DeModulator::from(
                Self::INTERMEDIATE_FREQ,
                intermediate_fs as f64,
                Self::SIGNAL_MAX_FREQ,
            ),
            // resampler
            upsampler,
            downsampler,
            upsampler_for_radio_waves,
            downsampler_for_radio_waves,
            // buffer
            tmp_buffer: [vec![0.; buffer_size], vec![0.; buffer_size]],
            audio_in_buffer: [
                vec![0.; composite_buffer_size],
                vec![0.; composite_buffer_size],
            ],
            composite_signal: vec![0.; modulated_buffer_size],
            up_sampled_signal: vec![0.; modulated_buffer_size],
            modulate_signal: vec![0.; modulated_buffer_size],
            intermediate_signal: vec![0.; modulated_buffer_size],
            intermediate_signal_out: vec![0.; intermediate_buffer_size],
            demodulate_signal: vec![0.; intermediate_buffer_size],
            post_down_sample: vec![0.; composite_buffer_size],
            restored_signal_l: vec![0.; composite_buffer_size],
            restored_signal_r: vec![0.; composite_buffer_size],
        }
    }
    pub fn process(&mut self, input: &[f32], dst: &mut [f32]) {
        // de-interleave
        for (i, lr) in input.chunks_exact(2).enumerate() {
            unsafe {
                *self.tmp_buffer[0].get_unchecked_mut(i) = lr[0] as f64;
                *self.tmp_buffer[1].get_unchecked_mut(i) = lr[1] as f64;
            }
        }
        // up sample
        let _ = self.upsampler[0].process::<f64, f64>(
            Some(&self.tmp_buffer[0]),
            &mut self.audio_in_buffer[0],
        );
        let _ = self.upsampler[1].process::<f64, f64>(
            Some(&self.tmp_buffer[1]),
            &mut self.audio_in_buffer[1],
        );
        // composite
        self.composite.process(
            &self.audio_in_buffer[0],
            &self.audio_in_buffer[1],
            &mut self.composite_signal,
        );
        //
        unsafe {
            upsample(
                self.up_sampled_signal.as_mut_ptr(),
                self.composite_signal.as_ptr(),
                &raw mut self.upsampler_for_radio_waves,
            );
        }
        //
        self.modulator
            .process(&self.up_sampled_signal, &mut self.modulate_signal);
        // super heterodyne
        self.freq_converter
            .process(&self.modulate_signal, &mut self.intermediate_signal);
        self.bandpass_filter.process(
            &self.intermediate_signal,
            &mut self.intermediate_signal_out,
        );
        // de-modulate
        self.demodulator.process(
            &self.intermediate_signal_out,
            &mut self.demodulate_signal,
        );
        //
        unsafe {
            downsample(
                self.post_down_sample.as_mut_ptr(),
                self.demodulate_signal.as_ptr(),
                &raw mut self.downsampler_for_radio_waves,
            );
        }
        self.restore.process(
            &self.post_down_sample,
            &mut self.restored_signal_l,
            &mut self.restored_signal_r,
        );
        // down sample
        let _ = self.downsampler[0].process::<f64, f64>(
            Some(&self.restored_signal_l),
            &mut self.tmp_buffer[0],
        );
        let _ = self.downsampler[1].process::<f64, f64>(
            Some(&self.restored_signal_r),
            &mut self.tmp_buffer[1],
        );
        // interleave
        for (i, lr) in dst.chunks_exact_mut(2).enumerate() {
            unsafe {
                lr[0] = *self.tmp_buffer[0].get_unchecked(i) as f32;
                lr[1] = *self.tmp_buffer[1].get_unchecked(i) as f32;
            }
        }
    }
}
fn generate_resampler(f1: f64, f2: f64) -> libsoxr::Result<Soxr> {
    Soxr::create(
        f1,
        f2,
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
}

#[inline]
fn get_buffer_size(s1: usize, s2: usize, base_size: usize) -> usize {
    (s2 as f64 / s1 as f64 * base_size as f64 + 0.5).floor() as usize
}
