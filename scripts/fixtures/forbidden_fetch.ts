export async function badFetch() {
  const response = await fetch("https://example.com");
  return response.ok;
}
