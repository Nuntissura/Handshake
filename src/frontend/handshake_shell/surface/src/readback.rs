//! A frame read back from the device: the shell's frame rendered into an owner image, or the swapchain image of a
//! presented frame, copied into host-visible memory by the same submission and handed to the host once the owner
//! observed it finished. The headless path needs no window and no presentation, so Argus and visual tests read what the
//! shell draws on any machine with a Vulkan device (MT-214 inspection route "argus.screenshot by Vulkan readback").
//!
//! Nothing here waits unless asked to: [`PendingReadback::poll`] looks without waiting, which is what a loop thread
//! uses; [`PendingReadback::wait`] blocks its caller and is for tests and tools.

use std::cell::Cell;
use std::time::{Duration, Instant};

use handshake_vulkan_renderer::{
    AccessRefusal, AttachmentFormat, BufferMemory, BufferRequest, BufferUsage, Claim, Extent,
    GpuClient, ImageLayout, ImageRequest, ImageUsage, OwnerBuffer, OwnerImage, RecordAborted,
    RecordRefusal, ResourceRefusal, Submission, SubmissionObservation, SubmissionRefusal,
    SubmissionState, SubmitRefusal,
};

use crate::clear::{compare_texels, ClearFrame, CompareRefusal, Comparison};

/// What the device wrote into an image, row after row without padding.
#[derive(Clone, PartialEq, Eq)]
pub struct FrameReadback {
    /// The image's format.
    pub format: AttachmentFormat,
    /// Its size.
    pub extent: Extent,
    /// Its texels.
    pub texels: Vec<u8>,
}

impl std::fmt::Debug for FrameReadback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameReadback")
            .field("format", &self.format)
            .field("extent", &self.extent)
            .field("bytes", &self.texels.len())
            .finish()
    }
}

impl FrameReadback {
    /// Compare it with `frame`, texel by texel.
    ///
    /// # Errors
    ///
    /// [`CompareRefusal`] when its size is not that of its extent.
    pub fn compare(&self, frame: &ClearFrame) -> Result<Comparison, CompareRefusal> {
        let expected = frame.expected(self.format, self.extent);
        compare_texels(self.format, self.extent, &self.texels, &expected)
    }
}

/// Why no readback came back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadbackError {
    /// The owner made no image or buffer.
    Resource(ResourceRefusal),
    /// The owner recorded no command.
    Record(RecordRefusal),
    /// Nothing was submitted.
    Submit(SubmitRefusal),
    /// The owner could not look at the submission.
    Observe(SubmissionRefusal),
    /// The buffer could not be read.
    Access(AccessRefusal),
    /// The submission ended without completing: its device was lost, or it never ran.
    Ended(SubmissionObservation),
    /// The image has no transfer source use (a swapchain whose surface does not offer it).
    NotReadable,
    /// [`PendingReadback::wait`] ran out of time.
    TimedOut,
}

impl ReadbackError {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::Resource(_) => "readback.resource",
            Self::Record(_) => "readback.record",
            Self::Submit(_) => "readback.submit",
            Self::Observe(_) => "readback.observe",
            Self::Access(_) => "readback.access",
            Self::Ended(_) => "readback.ended",
            Self::NotReadable => "readback.not_readable",
            Self::TimedOut => "readback.timed_out",
        }
    }
}

/// A readback on its way: the submission that writes it, and the owner resources it writes, which stay alive until it
/// is taken or dropped. Dropping it releases them; the owner destroys them after the work.
pub struct PendingReadback {
    _image: Option<OwnerImage>,
    buffer: OwnerBuffer,
    submission: Submission,
    format: AttachmentFormat,
    extent: Extent,
}

impl std::fmt::Debug for PendingReadback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingReadback")
            .field("format", &self.format)
            .field("extent", &self.extent)
            .finish_non_exhaustive()
    }
}

impl PendingReadback {
    pub(crate) fn new(
        image: Option<OwnerImage>,
        buffer: OwnerBuffer,
        submission: Submission,
        format: AttachmentFormat,
        extent: Extent,
    ) -> Self {
        Self {
            _image: image,
            buffer,
            submission,
            format,
            extent,
        }
    }

    /// The submission that writes it.
    pub fn submission(&self) -> &Submission {
        &self.submission
    }

    /// The readback, once the owner observed its submission completed and no other work of `client` can still write the
    /// buffer; `None` until then. Does not wait.
    ///
    /// # Errors
    ///
    /// [`ReadbackError`]: the submission did not complete, or the buffer cannot be read.
    pub fn poll(&self, client: &GpuClient) -> Result<Option<FrameReadback>, ReadbackError> {
        let state = match self.submission.state() {
            SubmissionState::Pending => match client.observe(&self.submission) {
                Ok(state) => state,
                Err(SubmissionRefusal::AlreadyResolved) => self.submission.state(),
                Err(refusal) => return Err(ReadbackError::Observe(refusal)),
            },
            finished => finished,
        };
        match state {
            SubmissionState::Pending => Ok(None),
            SubmissionState::Finished(SubmissionObservation::Completed) => {
                let bytes = self
                    .extent
                    .texels()
                    .saturating_mul(u64::from(self.format.texel_bytes()));
                let mut texels = vec![0; usize::try_from(bytes).unwrap_or(usize::MAX)];
                match self.buffer.read(0, &mut texels) {
                    Ok(()) => Ok(Some(FrameReadback {
                        format: self.format,
                        extent: self.extent,
                        texels,
                    })),
                    // Other work of the same client may still use the buffer's device memory.
                    Err(AccessRefusal::InFlight { .. }) => Ok(None),
                    Err(refusal) => Err(ReadbackError::Access(refusal)),
                }
            }
            SubmissionState::Finished(ended) => Err(ReadbackError::Ended(ended)),
        }
    }

    /// [`Self::poll`] until the readback is there or `timeout` has passed. Blocks the calling thread, each wait bounded
    /// by the owner's own deadline: for tests and tools, never on a loop's frame path.
    ///
    /// # Errors
    ///
    /// [`ReadbackError`], [`ReadbackError::TimedOut`] after `timeout`.
    pub fn wait(self, client: &GpuClient, timeout: Duration) -> Result<FrameReadback, ReadbackError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(readback) = self.poll(client)? {
                return Ok(readback);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(ReadbackError::TimedOut);
            }
            if self.submission.state() == SubmissionState::Pending {
                let _ = client.wait(&self.submission, left.min(Duration::from_millis(50)));
            } else {
                std::thread::sleep(left.min(Duration::from_millis(2)));
            }
        }
    }
}

/// Render `frame` into a new owner image of `format` and `extent` under `claim`, and copy it to a new readback buffer
/// in the same submission. The image is cleared from `UNDEFINED`, left in `TRANSFER_SRC_OPTIMAL` and copied from there.
///
/// # Errors
///
/// [`ReadbackError::Resource`], [`ReadbackError::Record`] or [`ReadbackError::Submit`]: nothing is on its way.
pub fn render_offscreen(
    client: &GpuClient,
    claim: &Claim,
    frame: &ClearFrame,
    format: AttachmentFormat,
    extent: Extent,
) -> Result<PendingReadback, ReadbackError> {
    let request = ImageRequest {
        format,
        extent,
        usage: ImageUsage::OFFSCREEN,
    };
    let image = client
        .create_image(claim, &request)
        .map_err(ReadbackError::Resource)?;
    let buffer = readback_buffer(client, claim, format, extent)?;
    let refused = Cell::new(None);
    let submitted = client.submit(claim, |recording| {
        frame
            .record(
                recording,
                &image,
                format,
                ImageLayout::Undefined,
                ImageLayout::TransferSrc,
            )
            .and_then(|()| {
                recording.read_back(
                    &image,
                    ImageLayout::TransferSrc,
                    &buffer,
                    ImageLayout::TransferSrc,
                )
            })
            .map_err(|refusal| {
                refused.set(Some(refusal));
                RecordAborted
            })
    });
    match submitted {
        Ok(submission) => Ok(PendingReadback::new(
            Some(image),
            buffer,
            submission,
            format,
            extent,
        )),
        Err(SubmitRefusal::RecordAborted) => Err(refused
            .get()
            .map_or(ReadbackError::Submit(SubmitRefusal::RecordAborted), ReadbackError::Record)),
        Err(refusal) => Err(ReadbackError::Submit(refusal)),
    }
}

/// A host-readable buffer for every texel of an image of `format` and `extent`, charged as staging memory to `claim`.
pub(crate) fn readback_buffer(
    client: &GpuClient,
    claim: &Claim,
    format: AttachmentFormat,
    extent: Extent,
) -> Result<OwnerBuffer, ReadbackError> {
    let request = BufferRequest {
        size: extent
            .texels()
            .saturating_mul(u64::from(format.texel_bytes())),
        usage: BufferUsage::READBACK,
        memory: BufferMemory::Readback,
    };
    client
        .create_buffer(claim, &request)
        .map_err(ReadbackError::Resource)
}
