use bytes::Bytes;
use hyper::Body;

pub struct Form<T> {
    pub(crate) value: T,
}

impl<T> Form<T> {
    pub fn new(value: T) -> Self {
        Form { value }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn take(self) -> T {
        self.value
    }
}

pub struct Html {
    pub(crate) body: Body,
}

impl Html {
    pub fn new(body: impl Into<Body>) -> Self {
        Html { body: body.into() }
    }
}

pub struct Json<T> {
    pub(crate) value: T,
}

impl<T> Json<T> {
    pub fn new(value: T) -> Self {
        Json { value }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn take(self) -> T {
        self.value
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

    pub fn value(&self) -> &Bytes {
        &self.body
    }

    pub fn take(self) -> Bytes {
        self.body
    }
}
