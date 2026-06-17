import { FormEvent, useEffect, useMemo, useState } from "react";
import {
  getUserManualPage,
  listUserManualAccessPoints,
  listUserManualPages,
  searchUserManual,
  type UserManualAccessPoint,
  type UserManualPageResponse,
  type UserManualPageSummary,
  type UserManualSearchHit,
  type UserManualSearchResponse,
} from "../lib/api";

type Props = {
  initialSlug?: string;
  initialSearchQuery?: string;
  searchRequestId?: number;
  panelStableId?: string;
};

const DEFAULT_PANEL_STABLE_ID = "hs-usermanual-panel";
const DIAGNOSTICS_ACCESS_POINT_ID = "ap.diagnostics.manual_tab";
const PAGE_INDEX_LIMIT = 100;
const SEARCH_LIMIT = 25;

function errorMessage(err: unknown, fallback: string): string {
  return err instanceof Error ? err.message : fallback;
}

function testIdToken(value: string): string {
  return value.replace(/[^a-zA-Z0-9_.:-]/g, "-");
}

function searchHitKind(hit: UserManualSearchHit): string {
  return hit.kind ?? hit.result_kind ?? "result";
}

function searchHitPageSlug(hit: UserManualSearchHit): string | null {
  const slug = hit.page_slug?.trim();
  return slug && slug.length > 0 ? slug : null;
}

function bodyLines(markdown: string): string[] {
  return markdown
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

export function UserManualPanel({
  initialSlug,
  initialSearchQuery = "",
  searchRequestId = 0,
  panelStableId = DEFAULT_PANEL_STABLE_ID,
}: Props) {
  const [pages, setPages] = useState<UserManualPageSummary[]>([]);
  const [manualVersion, setManualVersion] = useState<string | null>(null);
  const [accessPoints, setAccessPoints] = useState<UserManualAccessPoint[]>([]);
  const [indexLoading, setIndexLoading] = useState(true);
  const [indexError, setIndexError] = useState<string | null>(null);
  const [selectedSlug, setSelectedSlug] = useState<string | null>(initialSlug ?? null);
  const [pageResponse, setPageResponse] = useState<UserManualPageResponse | null>(null);
  const [pageLoading, setPageLoading] = useState(false);
  const [pageError, setPageError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState(initialSearchQuery);
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [searchResponse, setSearchResponse] = useState<UserManualSearchResponse | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIndexLoading(true);
    setIndexError(null);

    Promise.all([
      listUserManualAccessPoints(),
      listUserManualPages({ limit: PAGE_INDEX_LIMIT }),
    ])
      .then(([accessPointResponse, pagesResponse]) => {
        if (cancelled) return;
        setAccessPoints(accessPointResponse.access_points);
        setPages(pagesResponse.pages);
        setManualVersion(pagesResponse.manual_version);

        const diagnosticsTarget =
          accessPointResponse.access_points.find(
            (point) => point.access_point_id === DIAGNOSTICS_ACCESS_POINT_ID,
          )?.target_page_slug ?? null;
        const nextSlug = initialSlug ?? diagnosticsTarget ?? pagesResponse.pages[0]?.slug ?? null;
        if (initialSlug) {
          setSelectedSlug(initialSlug);
        } else {
          setSelectedSlug((current) => current ?? nextSlug);
        }
      })
      .catch((err) => {
        if (!cancelled) setIndexError(errorMessage(err, "Failed to load UserManual index"));
      })
      .finally(() => {
        if (!cancelled) setIndexLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [initialSlug]);

  useEffect(() => {
    if (!selectedSlug) {
      return;
    }
    let cancelled = false;
    setPageLoading(true);
    setPageError(null);
    setPageResponse(null);

    getUserManualPage(selectedSlug)
      .then((response) => {
        if (!cancelled) setPageResponse(response);
      })
      .catch((err) => {
        if (!cancelled) {
          setPageResponse(null);
          setPageError(errorMessage(err, "Failed to load UserManual page"));
        }
      })
      .finally(() => {
        if (!cancelled) setPageLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [selectedSlug]);

  useEffect(() => {
    const trimmed = initialSearchQuery.trim();
    setSearchQuery(initialSearchQuery);
    if (trimmed.length === 0) {
      return;
    }

    let cancelled = false;
    setSearchLoading(true);
    setSearchError(null);
    setSearchResponse(null);
    searchUserManual(trimmed, SEARCH_LIMIT)
      .then((response) => {
        if (!cancelled) setSearchResponse(response);
      })
      .catch((err) => {
        if (!cancelled) setSearchError(errorMessage(err, "Failed to search UserManual"));
      })
      .finally(() => {
        if (!cancelled) setSearchLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [initialSearchQuery, searchRequestId]);

  const selectedPageSummary = useMemo(
    () => pages.find((page) => page.slug === selectedSlug) ?? null,
    [pages, selectedSlug],
  );
  const activePageResponse = pageResponse?.page.slug === selectedSlug ? pageResponse : null;

  const runSearch = async (event?: FormEvent) => {
    event?.preventDefault();
    const trimmed = searchQuery.trim();
    if (trimmed.length === 0) {
      setSearchResponse(null);
      setSearchError("Enter a UserManual search term.");
      return;
    }
    setSearchLoading(true);
    setSearchError(null);
    setSearchResponse(null);
    try {
      setSearchResponse(await searchUserManual(trimmed, SEARCH_LIMIT));
    } catch (err) {
      setSearchError(errorMessage(err, "Failed to search UserManual"));
    } finally {
      setSearchLoading(false);
    }
  };

  const openPage = (slug: string) => {
    setSelectedSlug(slug);
  };

  return (
    <section
      className="content-card usermanual-panel"
      data-testid="usermanual-panel"
      data-stable-id={panelStableId}
      data-selected-slug={selectedSlug ?? ""}
    >
      <div className="card-header">
        <div>
          <p className="app-eyebrow">UserManual</p>
          <h2>UserManual</h2>
          <p className="muted">
            {manualVersion ? `Manual ${manualVersion}` : "Manual index"}
            {selectedPageSummary ? ` / ${selectedPageSummary.status}` : ""}
          </p>
        </div>
      </div>

      <div
        className="usermanual-layout"
        data-testid="usermanual-layout"
        data-manual-layout="index-page-search"
      >
        <aside className="usermanual-sidebar" aria-label="UserManual pages">
          <form className="usermanual-search" onSubmit={(event) => void runSearch(event)}>
            <label className="muted small" htmlFor="usermanual-search-input">
              Search
            </label>
            <div className="filter-actions">
              <input
                id="usermanual-search-input"
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder="Search UserManual"
                data-testid="usermanual-search-input"
                data-stable-id="usermanual-search-input"
              />
              <button
                type="submit"
                disabled={searchLoading}
                data-testid="usermanual-search-submit"
                data-stable-id="usermanual-search-submit"
              >
                {searchLoading ? "Searching..." : "Search"}
              </button>
            </div>
          </form>

          {searchError ? (
            <div
              className="error small"
              role="alert"
              data-testid="usermanual-search-error"
              data-stable-id="usermanual-search-error"
            >
              {searchError}
            </div>
          ) : null}

          {searchResponse ? (
            <section
              className="usermanual-search-results"
              aria-label="UserManual search results"
              data-testid="usermanual-search-results"
              data-stable-id="usermanual-search-results"
            >
              <h3>Search Results</h3>
              {searchResponse.results.length === 0 ? (
                <p className="muted">No matching manual entries.</p>
              ) : (
                searchResponse.results.map((hit) => {
                  const kind = searchHitKind(hit);
                  const pageSlug = searchHitPageSlug(hit);
                  const token = testIdToken(pageSlug ?? hit.result_ref);
                  const resultBody = (
                    <>
                      <strong>{hit.title}</strong>
                      <span className="muted small">
                        {kind} / {pageSlug ?? hit.result_ref}
                      </span>
                      <span>{hit.excerpt}</span>
                    </>
                  );
                  return pageSlug ? (
                    <button
                      key={`${kind}-${hit.result_ref}-${pageSlug}`}
                      type="button"
                      className="job-card"
                      onClick={() => openPage(pageSlug)}
                      data-testid={`usermanual-search-result-${token}`}
                      data-stable-id={`usermanual-search-result-${token}`}
                      data-page-backed="true"
                      data-result-ref={hit.result_ref}
                    >
                      {resultBody}
                    </button>
                  ) : (
                    <div
                      key={`${kind}-${hit.result_ref}-non-page`}
                      className="job-card usermanual-search-result usermanual-search-result--non-page"
                      data-testid={`usermanual-search-result-${token}`}
                      data-stable-id={`usermanual-search-result-${token}`}
                      data-page-backed="false"
                      data-result-ref={hit.result_ref}
                    >
                      {resultBody}
                    </div>
                  );
                })
              )}
            </section>
          ) : null}

          <section
            className="usermanual-page-list"
            aria-label="UserManual page index"
            data-testid="usermanual-page-list"
            data-stable-id="usermanual-page-list"
          >
            <h3>Pages</h3>
            {indexLoading ? <p className="muted">Loading pages...</p> : null}
            {indexError ? (
              <div className="error small" role="alert" data-testid="usermanual-index-error">
                {indexError}
              </div>
            ) : null}
            {pages.map((page) => {
              const selected = page.slug === selectedSlug;
              return (
                <button
                  key={page.slug}
                  type="button"
                  className={`job-card ${selected ? "job-card--active" : ""}`}
                  aria-current={selected ? "page" : undefined}
                  onClick={() => openPage(page.slug)}
                  data-testid={`usermanual-page-link-${testIdToken(page.slug)}`}
                  data-stable-id={`usermanual-page-link-${testIdToken(page.slug)}`}
                >
                  <strong>{page.title}</strong>
                  <span className="muted small">{page.slug}</span>
                </button>
              );
            })}
          </section>

          <section
            className="usermanual-access-points"
            aria-label="UserManual access points"
            data-testid="usermanual-access-points"
            data-stable-id="usermanual-access-points"
          >
            <h3>Access Points</h3>
            {accessPoints.map((point) => (
              <div
                key={point.access_point_id}
                className="usermanual-access-point"
                data-testid={`usermanual-access-point-${testIdToken(point.stable_element_id)}`}
                data-stable-id={`usermanual-access-point-${testIdToken(point.access_point_id)}`}
                data-access-point-id={point.access_point_id}
                data-access-point-stable-id={point.stable_element_id}
                data-target-resolves={point.target_resolves ? "true" : "false"}
              >
                <strong>{point.host_surface}</strong>
                <span className="muted small">{point.stable_element_id}</span>
                <span>{point.target_page_slug}</span>
              </div>
            ))}
          </section>
        </aside>

        <article
          className="usermanual-page"
          data-testid="usermanual-page"
          data-stable-id="usermanual-page"
          aria-busy={pageLoading ? "true" : "false"}
        >
          {pageLoading ? <p className="muted">Loading page...</p> : null}
          {pageError ? (
            <div
              className="error"
              role="alert"
              data-testid="usermanual-page-error"
              data-stable-id="usermanual-page-error"
            >
              {pageError}
            </div>
          ) : null}
          {activePageResponse ? (
            <>
              <header className="usermanual-page__header">
                <p className="app-eyebrow">{activePageResponse.page.slug}</p>
                <h3>{activePageResponse.page.title}</h3>
                <p className="muted">
                  {activePageResponse.page.page_kind} / {activePageResponse.page.audience} /{" "}
                  {activePageResponse.page.content_hash}
                </p>
              </header>

              <div className="usermanual-sections" data-testid="usermanual-sections">
                {activePageResponse.sections.map((section) => (
                  <section
                    key={section.section_id}
                    className="usermanual-section"
                    data-testid={`usermanual-section-${testIdToken(section.section_id)}`}
                    data-stable-id={`usermanual-section-${testIdToken(section.section_id)}`}
                  >
                    <h4>{section.title}</h4>
                    {bodyLines(section.body_md).map((line, index) => (
                      <p key={`${section.section_id}-${index}`}>{line}</p>
                    ))}
                  </section>
                ))}
              </div>

              {activePageResponse.anchors.length > 0 ? (
                <div className="table-scroll usermanual-anchors" data-testid="usermanual-anchors">
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th>Kind</th>
                        <th>Target</th>
                        <th>Method</th>
                      </tr>
                    </thead>
                    <tbody>
                      {activePageResponse.anchors.map((anchor) => (
                        <tr key={anchor.anchor_id}>
                          <td>{anchor.anchor_kind}</td>
                          <td>{anchor.anchor_value}</td>
                          <td>{anchor.http_method ?? ""}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : null}
            </>
          ) : !pageLoading && !pageError ? (
            <p className="muted">No UserManual page selected.</p>
          ) : null}
        </article>
      </div>
    </section>
  );
}
