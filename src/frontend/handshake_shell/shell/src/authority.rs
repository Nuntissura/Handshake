//! Who may open the device (MT-214 AC214-10, NATIVE-RENDER-BOOT-001 to 008).
//!
//! The product binary opens the GPU owner only under the registered full capability `device.gpu.shell` and the real
//! kernel issuer's binding. Neither exists yet: the registry entry is MT-219 (not published), and the host-to-module
//! trust-port binding is described by NATIVE-RENDER-BOOT-007 as required semantics, "not an implemented API". So the
//! authority the product needs, [`IssuedAuthority`], has no value: it holds an uninhabited type, nothing in this
//! crate or its callers can make one, and [`resolve_product`] refuses with a typed outcome. When the issuer's
//! reviewed binding exists, its constructor is the only change here; no caller-supplied identity, serialized
//! decision, registry label or capability string can stand in for it (NATIVE-RENDER-BOOT-002).
//!
//! A test or example binary opens the device with a [`TestBinary`] declaration instead. It authorizes nothing, is never
//! the product, and says so in its receipt line ([`DeviceAuthority::receipt`]): `product=false device_authority=none`.
//! Only `examples/` and `tests/` of this crate declare one; `tests/source_scan.rs` fails on a declaration in `src/`
//! and on the product binary naming one.

use std::convert::Infallible;

/// The registered full capability the pre-account shell needs (NATIVE-RENDER-REG-001).
pub const CAPABILITY: &str = "device.gpu.shell";

/// What is missing for the product binary, exactly: the first is MT-219, the second the issuer of
/// NATIVE-RENDER-BOOT-002 and -007.
pub const MISSING: [&str; 2] = [
    "device.gpu.shell registered in the capability registry (MT-219, not published)",
    "kernel issuer's reviewed host-to-module trust-port binding (NATIVE-RENDER-BOOT-007, no API exists)",
];

/// The kernel issuer's binding for this process and shell generation. Uninhabited until the issuer exists.
#[derive(Clone, Copy, Debug)]
pub struct IssuedAuthority {
    never: Infallible,
}

impl IssuedAuthority {
    /// Anything an issued authority would give: there is none, so this is never called.
    pub fn unreachable<T>(&self) -> T {
        match self.never {}
    }
}

/// A test or example binary's own declaration that it opens the device without the product's authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TestBinary {
    name: &'static str,
}

impl TestBinary {
    /// Declare it, by the name its receipt carries.
    pub const fn declare(name: &'static str) -> Self {
        Self { name }
    }

    /// The name in the receipt.
    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// What a program was started under.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum DeviceAuthority {
    /// The product: `device.gpu.shell` under the kernel issuer.
    Issued(IssuedAuthority),
    /// A test or example binary: nothing is authorized.
    Test(TestBinary),
}

impl DeviceAuthority {
    /// Whether this is the product's authority.
    pub fn is_product(&self) -> bool {
        matches!(self, Self::Issued(_))
    }

    /// The first line a binary prints: what it runs under. A test binary says it is not the product and has no device
    /// authority.
    pub fn receipt(&self) -> String {
        match self {
            Self::Issued(issued) => issued.unreachable(),
            Self::Test(test) => format!(
                "HSK_SHELL_RECEIPT binary=test name={} product=false device_authority=none capability_required={CAPABILITY}",
                test.name()
            ),
        }
    }
}

/// Why the product did not start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StartupRefusal {
    /// No `device.gpu.shell` under the kernel issuer: see [`MISSING`].
    NoDeviceAuthority,
}

impl StartupRefusal {
    /// A stable name, for diagnostics.
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoDeviceAuthority => "startup.no_device_authority",
        }
    }

    /// The line the product prints when it refuses.
    pub fn line(self) -> String {
        format!(
            "HSK_SHELL_REFUSED code={} capability={CAPABILITY} missing=\"{}; {}\"",
            self.code(),
            MISSING[0],
            MISSING[1]
        )
    }
}

/// The product's authority, from the kernel issuer. Refuses until the issuer exists.
///
/// # Errors
///
/// [`StartupRefusal::NoDeviceAuthority`].
pub fn resolve_product() -> Result<IssuedAuthority, StartupRefusal> {
    Err(StartupRefusal::NoDeviceAuthority)
}

/// Start the product: `run` is called, with the authority, only when the issuer gave one. It opens the device, so it is
/// never called without the gate having passed.
///
/// # Errors
///
/// [`StartupRefusal`], with `run` not called.
pub fn run_product<R>(run: impl FnOnce(DeviceAuthority) -> R) -> Result<R, StartupRefusal> {
    resolve_product().map(|issued| run(DeviceAuthority::Issued(issued)))
}
