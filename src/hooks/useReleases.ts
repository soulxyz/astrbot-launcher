import { useState, useCallback } from 'react';
import { message } from '../antdStatic';
import { api } from '../api';
import type { GitHubRelease } from '../types';
import { getErrorMessage } from '../utils';

/**
 * Hook for fetching GitHub releases.
 *
 * Caching/TTL is handled by the backend via `version_list.json`.
 */
export function useReleases() {
  const [releases, setReleases] = useState<GitHubRelease[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchReleases = useCallback(async (forceRefresh = false) => {
    setLoading(true);
    try {
      const r = await api.fetchReleases(forceRefresh);
      setReleases(r);
      return r;
    } catch (e: unknown) {
      message.error(getErrorMessage(e));
      return [];
    } finally {
      setLoading(false);
    }
  }, []);

  return { releases, loading, fetchReleases, setReleases };
}
