type RevalidateType = 'article' | 'ingredient' | 'shop' | 'about' | 'gallery' | 'all';

type RevalidatePayload = {
  type?: RevalidateType;
  slug?: string | null;
  paths?: string[];
  tags?: string[];
};

const BLOG_REVALIDATE_URL =
  import.meta.env.VITE_BLOG_REVALIDATE_URL || 'https://dima-fomin.pl/api/revalidate';

const BLOG_REVALIDATE_SECRET = import.meta.env.VITE_BLOG_REVALIDATE_SECRET || '';

export async function revalidateSite(payload: RevalidatePayload): Promise<void> {
  if (!BLOG_REVALIDATE_URL) return;

  try {
    const response = await fetch(BLOG_REVALIDATE_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        ...payload,
        secret: BLOG_REVALIDATE_SECRET || undefined,
      }),
    });

    if (!response.ok) {
      console.warn('Site revalidation failed', response.status, await response.text().catch(() => ''));
    }
  } catch (error) {
    console.warn('Site revalidation failed', error);
  }
}
