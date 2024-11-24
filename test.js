
// const FS = 180.0;
const FS = 192/4;
var calc_fe = (fs,f) => fs -f;
var get_upper_freq_hi = (fc) => 2*fc - 10.7 + 0.1;
var get_upper_freq_lo = (fc) => 2*fc - 10.7 - 0.1;

var get_fe_range = (fs,freq) => {
  let fh = get_upper_freq_hi(freq);
  let fl = get_upper_freq_lo(freq);
  let i = 0;
  while( Math.abs((++i)*fs - fh) > fs/2);
  let j = 0;
  while( Math.abs((++j)*fs - fl) > fs/2);
  return [(i)*fs - fh, (j)*fs - fl];
}
[...Array(200)].map((_,i) => {
  console.log(76 + i/200 * 20,get_fe_range(FS,76 + i/200 * 20));
})