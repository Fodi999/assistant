import type { AffiliateNetwork, AffiliateOffer } from '../types/admin';

interface AffiliateOfferCardsProps {
  offers?: AffiliateOffer[];
}

const networks: AffiliateNetwork[] = ['amazon', 'allegro', 'ceneo', 'awin', 'custom'];

export function AffiliateOfferCards({ offers = [] }: AffiliateOfferCardsProps) {
  return (
    <div className="offer-card-grid">
      {networks.map((network) => {
        const offer = offers.find((item) => item.network === network);
        return (
          <article key={network} className={offer?.isActive ? 'offer-card active' : 'offer-card'}>
            <strong>{network.toUpperCase()}</strong>
            {offer ? (
              <>
                <span>{offer.merchant}</span>
                <p>{offer.price ? `${offer.price.toLocaleString('ru-RU')} ${offer.currency}` : 'цена не задана'}</p>
                <small>{offer.commissionPercent ?? 0}% / cookie {offer.cookieDays ?? 0} дн.</small>
              </>
            ) : (
              <>
                <span>Нет оффера</span>
                <p>Подключить</p>
                <small>готово для импорта</small>
              </>
            )}
          </article>
        );
      })}
    </div>
  );
}
