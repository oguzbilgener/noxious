// Define "integration tests" here so we have private access, and we don't have to recompile the library.

#[cfg(test)]
mod proxy;
#[cfg(test)]
mod socket_mocks;