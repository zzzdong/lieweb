use bytes::Bytes;

pub struct Form<T> {
    pub(crate) body: T,
}

impl<T> Form<T> {
    pub fn new(body: T) -> Self {
        Form { body }
    }
}

pub struct Html<T> {
    pub(crate) body: T,
}

impl<T> Html<T>
where
    T: Send,
{
    pub fn new(body: T) -> Self {
        Html { body }
    }
}

pub struct Json<T> {
    pub(crate) body: T,
}

impl<T> Json<T> {
    pub fn new(body: T) -> Self {
        Json { body }
    }
}

pub struct StreamBody<S> {
    pub(crate) s: S,
    pub(crate) content_type: mime::Mime,
}

impl<S, B, E> StreamBody<S>
where
    S: futures::Stream<Item = Result<B, E>> + Send + 'static,
    B: Into<Bytes> + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    pub fn new(s: S, content_type: mime::Mime) -> Self {
        StreamBody { s, content_type }
    }
}

pub struct BytesBody {
    pub(crate) body: Bytes,
    pub(crate) content_type: mime::Mime,
}

impl BytesBody {
    pub fn new(body: impl Into<Bytes>, content_type: mime::Mime) -> Self {
        BytesBody {
            body: body.into(),
            content_type,
        }
    }
}
