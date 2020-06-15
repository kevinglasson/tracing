use crate::{
    field::RecordFields,
    fmt::{format, FormatEvent, FormatFields, MakeWriter},
    layer::{self, Context},
    registry::{LookupSpan, SpanRef},
};
use std::{any::TypeId, cell::RefCell, fmt, io, marker::PhantomData, ops::Deref};
use tracing_core::{
    span::{Attributes, Id, Record},
    Event, Subscriber,
};

/// A [`Layer`] that logs formatted representations of `tracing` events.
///
/// ## Examples
///
/// Constructing a layer with the default configuration:
///
/// ```rust
/// use tracing_subscriber::{fmt, Registry};
/// use tracing_subscriber::prelude::*;
///
/// let subscriber = Registry::default()
///     .with(fmt::Layer::default());
///
/// tracing::subscriber::set_global_default(subscriber).unwrap();
/// ```
///
/// Overriding the layer's behavior:
///
/// ```rust
/// use tracing_subscriber::{fmt, Registry};
/// use tracing_subscriber::prelude::*;
///
/// let fmt_layer = fmt::layer()
///    .with_target(false) // don't include event targets when logging
///    .with_level(false); // don't include event levels when logging
///
/// let subscriber = Registry::default().with(fmt_layer);
/// # tracing::subscriber::set_global_default(subscriber).unwrap();
/// ```
///
/// Setting a custom event formatter:
///
/// ```rust
/// use tracing_subscriber::fmt::{self, format, time};
/// use tracing_subscriber::prelude::*;
///
/// let fmt = format().with_timer(time::Uptime::default());
/// let fmt_layer = fmt::layer()
///     .event_format(fmt)
///     .with_target(false);
/// # let subscriber = fmt_layer.with_subscriber(tracing_subscriber::registry::Registry::default());
/// # tracing::subscriber::set_global_default(subscriber).unwrap();
/// ```
///
/// [`Layer`]: ../layer/trait.Layer.html
#[derive(Debug)]
pub struct Layer<
    S,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    make_writer: W,
    fmt_fields: N,
    fmt_event: E,
    _inner: PhantomData<S>,
}

/// A builder for [`Layer`](struct.Layer.html) that logs formatted representations of `tracing`
/// events and spans.
///
/// **Note**: As of `tracing-subscriber` 0.2.4, the separate builder type is now
/// deprecated, as the `Layer` type itself supports all the builder's
/// configuration methods. This is now an alias for `Layer`.
#[deprecated(
    since = "0.2.4",
    note = "a separate layer builder type is not necessary, `Layer`s now support configuration"
)]
pub type LayerBuilder<
    S,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> = Layer<S, N, E, W>;

impl<S> Layer<S> {
    /// Returns a new [`LayerBuilder`](type.LayerBuilder.html) for configuring a `Layer`.
    #[deprecated(
        since = "0.2.4",
        note = "a separate layer builder is not necessary, use `Layer::new`/`Layer::default` instead"
    )]
    #[allow(deprecated)]
    pub fn builder() -> LayerBuilder<S> {
        Layer::default()
    }

    /// Returns a new [`Layer`](struct.Layer.html) with the default configuration.
    pub fn new() -> Self {
        Self::default()
    }
}

// This needs to be a seperate impl block because they place different bounds on the type paramaters.
impl<S, N, E, W> Layer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    /// Sets the [event formatter][`FormatEvent`] that the layer will use to
    /// format events.
    ///
    /// The event formatter may be any type implementing the [`FormatEvent`]
    /// trait, which is implemented for all functions taking a [`FmtContext`], a
    /// `&mut dyn Write`, and an [`Event`].
    ///
    /// # Examples
    ///
    /// Setting a type implementing [`FormatEvent`] as the formatter:
    /// ```rust
    /// use tracing_subscriber::fmt::{self, format};
    ///
    /// let layer = fmt::layer()
    ///     .event_format(format().compact());
    /// # // this is necessary for type inference.
    /// # use tracing_subscriber::Layer as _;
    /// # let _ = layer.with_subscriber(tracing_subscriber::registry::Registry::default());
    /// ```
    /// [event formatter]: ../format/trait.FormatEvent.html
    /// [`FmtContext`]: ../struct.FmtContext.html
    /// [`Event`]: https://docs.rs/tracing/latest/tracing/struct.Event.html
    pub fn event_format<E2>(self, e: E2) -> Layer<S, N, E2, W>
    where
        E2: FormatEvent<S, N> + 'static,
    {
        Layer {
            fmt_fields: self.fmt_fields,
            fmt_event: e,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }
}

// This needs to be a seperate impl block because they place different bounds on the type paramaters.
impl<S, N, E, W> Layer<S, N, E, W> {
    /// Sets the [`for<'writer> MakeWriter<'writer>`] that the [`Layer`] being built will use to write events.
    ///
    /// # Examples
    ///
    /// Using `stderr` rather than `stdout`:
    ///
    /// ```rust
    /// use std::io;
    /// use tracing_subscriber::fmt;
    ///
    /// let layer = fmt::layer()
    ///     .with_writer(io::stderr);
    /// # // this is necessary for type inference.
    /// # use tracing_subscriber::Layer as _;
    /// # let _ = layer.with_subscriber(tracing_subscriber::registry::Registry::default());
    /// ```
    ///
    /// [`for<'writer> MakeWriter<'writer>`]: ../fmt/trait.for<'writer> MakeWriter<'writer>.html
    /// [`Layer`]: ../layer/trait.Layer.html
    pub fn with_writer<W2>(self, make_writer: W2) -> Layer<S, N, E, W2>
    where
        W2: for<'writer> MakeWriter<'writer> + 'static,
    {
        Layer {
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
            make_writer,
            _inner: self._inner,
        }
    }
}

impl<S, N, L, T, W> Layer<S, N, format::Format<L, T>, W>
where
    N: for<'writer> FormatFields<'writer> + 'static,
{
    /// Use the given [`timer`] for span and event timestamps.
    ///
    /// See [`time`] for the provided timer implementations.
    ///
    /// Note that using the `chrono` feature flag enables the
    /// additional time formatters [`ChronoUtc`] and [`ChronoLocal`].
    ///
    /// [`time`]: ./time/index.html
    /// [`timer`]: ./time/trait.FormatTime.html
    /// [`ChronoUtc`]: ./time/struct.ChronoUtc.html
    /// [`ChronoLocal`]: ./time/struct.ChronoLocal.html
    pub fn with_timer<T2>(self, timer: T2) -> Layer<S, N, format::Format<L, T2>, W> {
        Layer {
            fmt_event: self.fmt_event.with_timer(timer),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Do not emit timestamps with spans and event.
    pub fn without_time(self) -> Layer<S, N, format::Format<L, ()>, W> {
        Layer {
            fmt_event: self.fmt_event.without_time(),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Enable ANSI encoding for formatted events.
    #[cfg(feature = "ansi")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ansi")))]
    pub fn with_ansi(self, ansi: bool) -> Layer<S, N, format::Format<L, T>, W> {
        Layer {
            fmt_event: self.fmt_event.with_ansi(ansi),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Layer<S, N, format::Format<L, T>, W> {
        Layer {
            fmt_event: self.fmt_event.with_target(display_target),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Sets whether or not an event's level is displayed.
    pub fn with_level(self, display_level: bool) -> Layer<S, N, format::Format<L, T>, W> {
        Layer {
            fmt_event: self.fmt_event.with_level(display_level),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Sets the layer being built to use a [less verbose formatter](../fmt/format/struct.Compact.html).
    pub fn compact(self) -> Layer<S, N, format::Format<format::Compact, T>, W>
    where
        N: for<'writer> FormatFields<'writer> + 'static,
    {
        Layer {
            fmt_event: self.fmt_event.compact(),
            fmt_fields: self.fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }

    /// Sets the layer being built to use a [JSON formatter](../fmt/format/struct.Json.html).
    ///
    /// The full format includes fields from all entered spans.
    ///
    /// # Example Output
    ///
    /// ```ignore,json
    /// {"timestamp":"Feb 20 11:28:15.096","level":"INFO","target":"mycrate","fields":{"message":"some message", "key": "value"}}
    /// ```
    ///
    /// # Options
    ///
    /// - [`Layer::flatten_event`] can be used to enable flattening event fields into the root
    /// object.
    ///
    /// [`Layer::flatten_event`]: #method.flatten_event
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub fn json(self) -> Layer<S, format::JsonFields, format::Format<format::Json, T>, W> {
        Layer {
            fmt_event: self.fmt_event.json(),
            fmt_fields: format::JsonFields::new(),
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }
}

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
impl<S, T, W> Layer<S, format::JsonFields, format::Format<format::Json, T>, W> {
    /// Sets the JSON layer being built to flatten event metadata.
    ///
    /// See [`format::Json`](../fmt/format/struct.Json.html)
    pub fn flatten_event(
        self,
        flatten_event: bool,
    ) -> Layer<S, format::JsonFields, format::Format<format::Json, T>, W> {
        Layer {
            fmt_event: self.fmt_event.flatten_event(flatten_event),
            fmt_fields: format::JsonFields::new(),
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }
}

impl<S, N, E, W> Layer<S, N, E, W> {
    /// Sets the field formatter that the layer being built will use to record
    /// fields.
    pub fn fmt_fields<N2>(self, fmt_fields: N2) -> Layer<S, N2, E, W>
    where
        N2: for<'writer> FormatFields<'writer> + 'static,
    {
        Layer {
            fmt_event: self.fmt_event,
            fmt_fields,
            make_writer: self.make_writer,
            _inner: self._inner,
        }
    }
}

#[allow(deprecated)]
impl<S, N, E, W> LayerBuilder<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    /// Builds a [`Layer`] with the provided configuration.
    ///
    /// [`Layer`]: struct.Layer.html
    #[deprecated(
        since = "0.2.4",
        note = "`LayerBuilder` is no longer a separate type; this method is not necessary"
    )]
    pub fn finish(self) -> Layer<S, N, E, W> {
        self
    }
}

impl<S> Default for Layer<S> {
    fn default() -> Self {
        Layer {
            fmt_fields: format::DefaultFields::default(),
            fmt_event: format::Format::default(),
            make_writer: io::stdout,
            _inner: PhantomData,
        }
    }
}

impl<S, N, E, W> Layer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    #[inline]
    fn make_ctx<'a>(&'a self, ctx: Context<'a, S>) -> FmtContext<'a, S, N> {
        FmtContext {
            ctx,
            fmt_fields: &self.fmt_fields,
        }
    }
}

/// A formatted representation of a span's fields stored in its [extensions].
///
/// Because `FormattedFields` is generic over the type of the formatter that
/// produced it, multiple versions of a span's formatted fields can be stored in
/// the [`Extensions`][extensions] type-map. This means that when multiple
/// formatters are in use, each can store its own formatted representation
/// without conflicting.
///
/// [extensions]: ../registry/extensions/index.html
#[derive(Default)]
pub struct FormattedFields<E> {
    _format_event: PhantomData<fn(E)>,
    /// The formatted fields of a span.
    pub fields: String,
}

impl<E> FormattedFields<E> {
    /// Returns a new `FormattedFields`.
    pub fn new(fields: String) -> Self {
        Self {
            fields,
            _format_event: PhantomData,
        }
    }
}

impl<E> fmt::Debug for FormattedFields<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormattedFields")
            .field("fields", &self.fields)
            .finish()
    }
}

impl<E> fmt::Display for FormattedFields<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.fields)
    }
}

impl<E> Deref for FormattedFields<E> {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

// === impl FmtLayer ===

impl<S, N, E, W> layer::Layer<S> for Layer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    fn new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if extensions.get_mut::<FormattedFields<N>>().is_none() {
            let mut buf = String::new();
            if self.fmt_fields.format_fields(&mut buf, attrs).is_ok() {
                let fmt_fields = FormattedFields {
                    fields: buf,
                    _format_event: PhantomData::<fn(N)>,
                };
                extensions.insert(fmt_fields);
            }
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if let Some(FormattedFields { ref mut fields, .. }) =
            extensions.get_mut::<FormattedFields<N>>()
        {
            let _ = self.fmt_fields.add_fields(fields, values);
        } else {
            let mut buf = String::new();
            if self.fmt_fields.format_fields(&mut buf, values).is_ok() {
                let fmt_fields = FormattedFields {
                    fields: buf,
                    _format_event: PhantomData::<fn(N)>,
                };
                extensions.insert(fmt_fields);
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        thread_local! {
            static BUF: RefCell<String> = RefCell::new(String::new());
        }

        BUF.with(|buf| {
            let borrow = buf.try_borrow_mut();
            let mut a;
            let mut b;
            let mut buf = match borrow {
                Ok(buf) => {
                    a = buf;
                    &mut *a
                }
                _ => {
                    b = String::new();
                    &mut b
                }
            };

            let ctx = self.make_ctx(ctx);
            if self.fmt_event.format_event(&ctx, &mut buf, event).is_ok() {
                let mut writer = self.make_writer.make_writer();
                let _ = io::Write::write_all(&mut writer, buf.as_bytes());
            }

            buf.clear();
        });
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        // This `downcast_raw` impl allows downcasting a `fmt` layer to any of
        // its components (event formatter, field formatter, and `for<'writer> MakeWriter<'writer>`)
        // as well as to the layer's type itself. The potential use-cases for
        // this *may* be somewhat niche, though...
        match () {
            _ if id == TypeId::of::<Self>() => Some(self as *const Self as *const ()),
            _ if id == TypeId::of::<E>() => Some(&self.fmt_event as *const E as *const ()),
            _ if id == TypeId::of::<N>() => Some(&self.fmt_fields as *const N as *const ()),
            _ if id == TypeId::of::<W>() => Some(&self.make_writer as *const W as *const ()),
            _ => None,
        }
    }
}

/// Provides the current span context to a formatter.
pub struct FmtContext<'a, S, N> {
    pub(crate) ctx: Context<'a, S>,
    pub(crate) fmt_fields: &'a N,
}

impl<'a, S, N> fmt::Debug for FmtContext<'a, S, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FmtContext").finish()
    }
}

impl<'a, S, N> FormatFields<'a> for FmtContext<'a, S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_fields<R: RecordFields>(
        &self,
        writer: &'a mut dyn fmt::Write,
        fields: R,
    ) -> fmt::Result {
        self.fmt_fields.format_fields(writer, fields)
    }
}

impl<'a, S, N> FmtContext<'a, S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    /// Visits every span in the current context with a closure.
    ///
    /// The provided closure will be called first with the current span,
    /// and then with that span's parent, and then that span's parent,
    /// and so on until a root span is reached.
    pub fn visit_spans<E, F>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&SpanRef<'_, S>) -> Result<(), E>,
    {
        // visit all the current spans
        for span in self.ctx.scope() {
            f(&span)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::fmt::{
        self,
        format::{self, Format},
        layer::Layer as _,
        time,
    };
    use crate::Registry;
    use tracing_core::dispatcher::Dispatch;

    #[test]
    fn impls() {
        let f = Format::default().with_timer(time::Uptime::default());
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default();
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default().compact();
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());
        let _dispatch = Dispatch::new(subscriber);
    }

    #[test]
    fn fmt_layer_downcasts() {
        let f = format::Format::default();
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());

        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<fmt::Layer<Registry>>().is_some());
    }

    #[test]
    fn fmt_layer_downcasts_to_parts() {
        let f = format::Format::default();
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<format::DefaultFields>().is_some());
        assert!(dispatch.downcast_ref::<format::Format>().is_some())
    }

    #[test]
    fn is_lookup_span() {
        fn assert_lookup_span<T: for<'a> crate::registry::LookupSpan<'a>>(_: T) {}
        let fmt = fmt::Layer::default();
        let subscriber = fmt.with_subscriber(Registry::default());
        assert_lookup_span(subscriber)
    }
}
