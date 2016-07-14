
use std::fmt::Debug;

use num::{Complex, FromPrimitive, Signed, Zero};

use math_utils;
use array_utils;

use algorithm::FFTAlgorithm;

pub struct GoodThomasAlgorithm<T> {
    width: usize,
    //width_inverse: usize,
    width_size_fft: Box<FFTAlgorithm<T>>,

    height: usize,
    //height_inverse: usize,
    height_size_fft: Box<FFTAlgorithm<T>>,

    input_map: Vec<usize>,
    output_map: Vec<usize>,

    scratch: Vec<Complex<T>>,
}

impl<T> GoodThomasAlgorithm<T>
    where T: Signed + FromPrimitive + Copy + Debug
{
    pub fn new(n1: usize, n1_fft: Box<FFTAlgorithm<T>>, n2: usize, n2_fft: Box<FFTAlgorithm<T>>) -> Self {
        
        //compute the nultiplicative inverse of n1 mod n2 and vice versa
        let (gcd, mut n1_inverse, mut n2_inverse) = math_utils::extended_euclidean_algorithm(n1 as i64, n2 as i64);
        assert!(gcd == 1, "Invalid input n1 and n2 to Good-Thomas Algorithm: ({},{}): Inputs must be coprime", n1, n2);

        //n1_inverse or n2_inverse might be negative, make it positive
        if n1_inverse < 0 {
            n1_inverse += n2 as i64;
        }
        if n2_inverse < 0 {
            n2_inverse += n1 as i64;
        }

        GoodThomasAlgorithm {
            width: n1,
            //width_inverse: n1_inverse as usize,
            width_size_fft: n1_fft,

            height: n2,
            //height_inverse: n2_inverse as usize,
            height_size_fft: n2_fft,

            input_map: (0..n1 * n2).map(|i| (i % n1, i / n1)).map(|(x,y)| (x * n2 + y * n1) % (n1 * n2)).collect(),
            output_map: (0..n1 * n2)
                .map(|i| (i % n2, i / n2))
                .map(|(y,x)| (x * n2 * n2_inverse as usize + y * n1 * n1_inverse as usize) % (n1 * n2))
                .collect(),

            scratch: vec![Zero::zero(); n1 * n2],
        }
    }

    #[inline(never)]
    fn copy_from_input(&mut self, input: &[Complex<T>], output: &mut [Complex<T>]) {
        
        //copy the input into the output, reordering the elements via the "ruritanian mapping"
        /*for (y, buffer_chunk) in output.chunks_mut(self.width).enumerate() {
            for (x, buffer_element) in buffer_chunk.iter_mut().enumerate() {
                let input_index = self.get_input_index(x,y,input.len());

                *buffer_element = unsafe { *input.get_unchecked(input_index) };
            }
        }*/

        for (output_element, input_index) in output.iter_mut().zip(self.input_map.iter()) {
           *output_element = unsafe { *input.get_unchecked(*input_index) };
        }
    }

    /*#[inline(never)]
    fn get_input_index(&self, x: usize, y: usize, len: usize) -> usize {
        (x * self.height + y * self.width) % len
    }*/

    #[inline(never)]
    fn copy_transposed_scratch_to_output(&self, output: &mut [Complex<T>]) {

        //copy the buffer into the output, reordering the elements via the "CRT mapping"
        //note that self.scratch is currently transposed
        //so we're wolling a transpose into this copy
        /*for (x, buffer_chunk) in self.scratch.chunks(self.height).enumerate() {
            for (y, buffer_element) in buffer_chunk.iter().enumerate() {
                let output_index = self.get_output_index(x,y, output.len());

                unsafe { *output.get_unchecked_mut(output_index) = *buffer_element };
            }
        }*/

        for (scratch_element, output_index) in self.scratch.iter().zip(self.output_map.iter()) {
            unsafe { *output.get_unchecked_mut(*output_index) = *scratch_element };
        }
    }

    /*#[inline(never)]
    fn get_output_index(&self, x: usize, y: usize, len: usize) -> usize {
        (x * self.height * self.height_inverse + y * self.width * self.width_inverse) % len
    }*/
}

impl<T> FFTAlgorithm<T> for GoodThomasAlgorithm<T>
    where T: Signed + FromPrimitive + Copy + Debug
{
    /// Runs the FFT on the input `signal` array, placing the output in the 'spectrum' array
    fn process(&mut self, signal: &[Complex<T>], spectrum: &mut [Complex<T>]) {
        //copy the input into the spectrum
        self.copy_from_input(signal, spectrum);

        //run 'height' FFTs of size 'width' from the spectrum into scratch
        for (input, output) in spectrum.chunks(self.width).zip(self.scratch.chunks_mut(self.width)) {
            self.width_size_fft.process(input, output);
        }

        //transpose the scratch back into the spectrum to prepare for the next round of FFT
        array_utils::transpose(self.width, self.height, self.scratch.as_slice(), spectrum);

        //run 'width' FFTs of size 'height' from the spectrum back into scratch
        for (input, output) in spectrum.chunks(self.height).zip(self.scratch.chunks_mut(self.height)) {
            self.height_size_fft.process(input, output);
        }

        //we're done, copy to the output
        self.copy_transposed_scratch_to_output(spectrum);
    }
}