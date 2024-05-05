pub mod bss;
pub mod gen;
pub mod sampletypes;

/*
#[pin_project]
pub struct BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
{
    #[pin]
    stream: St,
}



impl<St, T> BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
{
    pub fn new(stream: St) -> Self {
        Self { stream }
    }

    pub fn map_chunks<R, F>(self, f: F) -> BufferedSampleStream<impl Stream<Item = Vec<R>>, R>
    where
        F: FnMut(Vec<T>) -> Vec<R>,
    {
        BufferedSampleStream::new(self.stream.map(f))
    }

    pub fn map_samples<R, F>(self, mut f: F) -> BufferedSampleStream<impl Stream<Item = Vec<R>>, R>
    where
        F: FnMut(T) -> R,
    {
        self.map_chunks(move |chunk| chunk.into_iter().map(|v| f(v)).collect())
    }
}

impl<St, T> BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
    T: Num,
{
    pub fn lift_complex(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<Complex<T>>>, Complex<T>> {
        self.map_samples(|v| Complex::new(v, T::zero()))
    }
}

impl<St, T> BufferedSampleStream<St, Complex<T>>
where
    St: Stream<Item = Vec<Complex<T>>>,
{
    pub fn map_sample_args<R, F>(
        self,
        mut f: F,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<Complex<R>>>, Complex<R>>
    where
        F: FnMut(T) -> R,
    {
        self.map_samples(move |v| Complex::new(f(v.re), f(v.im)))
    }

    pub fn realpart(self) -> BufferedSampleStream<impl Stream<Item = Vec<T>>, T> {
        self.map_samples(|v| v.re)
    }
}

impl<St> BufferedSampleStream<St, ComplexFloat>
where
    St: Stream<Item = Vec<ComplexFloat>>,
{
    pub fn convert_to_s16(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexS16>>, ComplexS16> {
        self.map_sample_args(float_to_s16)
    }

    pub fn convert_to_char(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexChar>>, ComplexChar> {
        self.map_sample_args(float_to_char)
    }
}

impl<St> BufferedSampleStream<St, ComplexS16>
where
    St: Stream<Item = Vec<ComplexS16>>,
{
    pub fn convert_to_float(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexFloat>>, ComplexFloat> {
        self.map_sample_args(s16_to_float)
    }

    pub fn convert_to_char(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexChar>>, ComplexChar> {
        self.map_sample_args(s16_to_char)
    }
}

impl<St> BufferedSampleStream<St, ComplexChar>
where
    St: Stream<Item = Vec<ComplexChar>>,
{
    pub fn convert_to_float(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexFloat>>, ComplexFloat> {
        self.map_sample_args(char_to_float)
    }

    pub fn convert_to_s16(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexS16>>, ComplexS16> {
        self.map_sample_args(char_to_s16)
    }
}

impl<St> BufferedSampleStream<St, Float>
where
    St: Stream<Item = Vec<Float>>,
{
    pub fn convert_to_s16(self) -> BufferedSampleStream<impl Stream<Item = Vec<S16>>, S16> {
        self.map_samples(float_to_s16)
    }

    pub fn convert_to_char(self) -> BufferedSampleStream<impl Stream<Item = Vec<Char>>, Char> {
        self.map_samples(float_to_char)
    }
}

impl<St> BufferedSampleStream<St, S16>
where
    St: Stream<Item = Vec<S16>>,
{
    pub fn convert_to_float(self) -> BufferedSampleStream<impl Stream<Item = Vec<Float>>, Float> {
        self.map_samples(s16_to_float)
    }

    pub fn convert_to_char(self) -> BufferedSampleStream<impl Stream<Item = Vec<Char>>, Char> {
        self.map_samples(s16_to_char)
    }
}

impl<St> BufferedSampleStream<St, Char>
where
    St: Stream<Item = Vec<Char>>,
{
    pub fn convert_to_float(self) -> BufferedSampleStream<impl Stream<Item = Vec<Float>>, Float> {
        self.map_samples(char_to_float)
    }

    pub fn convert_to_s16(self) -> BufferedSampleStream<impl Stream<Item = Vec<S16>>, S16> {
        self.map_samples(char_to_s16)
    }
}

impl<St, T> Stream for BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
{
    type Item = Vec<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.project().stream.poll_next(cx)
    }
}

impl<St, T> From<St> for BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
{
    fn from(stream: St) -> Self {
        Self { stream }
    }
}
*/
/*
pub struct Pipeline<T> {
    inner: Pin<Box<dyn Stream<Item = Vec<T>>>>,
}

impl<S, T> From<S> for Pipeline<T>
where
    S: Stream<Item = Vec<T>> + 'static,
{
    fn from(s: S) -> Self {
        Self { inner: Box::pin(s) }
    }
}

impl<T> Deref for Pipeline<T> {
    type Target = Pin<Box<dyn Stream<Item = Vec<T>>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Pipeline<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Pipeline<T>
where
    T: 'static,
{
    pub fn map_chunks<R>(self, f: impl FnMut(Vec<T>) -> Vec<R> + 'static) -> Pipeline<R> {
        Pipeline::from(self.inner.map(f))
    }

    pub fn map_values<R>(self, mut f: impl FnMut(T) -> R + 'static) -> Pipeline<R> {
        self.map_chunks(move |chunk| chunk.into_iter().map(|v| f(v)).collect())
    }
}

impl Pipeline<ComplexFloat> {
    pub fn convert_to_float(self) -> Self {
        self
    }

    pub fn convert_to_s16(self) -> Pipeline<ComplexS16> {
        self.map_values(lift_complex(float_to_s16))
    }

    pub fn convert_to_char(self) -> Pipeline<ComplexChar> {
        self.map_values(lift_complex(float_to_char))
    }
}
*/
/*

*/
