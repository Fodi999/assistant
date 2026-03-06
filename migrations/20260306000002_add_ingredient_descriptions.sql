-- Populate multilingual descriptions for top ingredients
-- These descriptions are used for SEO ingredient pages

UPDATE catalog_ingredients SET
  description_en = 'Salmon is a fatty fish rich in omega-3 fatty acids, widely used in European and Japanese cuisine. An excellent source of high-quality protein and vitamin D.',
  description_pl = 'Łosoś to tłusta ryba bogata w kwasy omega-3, szeroko stosowana w kuchni europejskiej i japońskiej. Doskonałe źródło pełnowartościowego białka i witaminy D.',
  description_ru = 'Лосось — жирная рыба, богатая омега-3 жирными кислотами, широко используется в европейской и японской кухне. Отличный источник белка и витамина D.',
  description_uk = 'Лосось — жирна риба, багата на омега-3 жирні кислоти, широко використовується в європейській та японській кухні. Чудове джерело білка та вітаміну D.'
WHERE LOWER(name_en) LIKE '%salmon%';

UPDATE catalog_ingredients SET
  description_en = 'Wheat flour is the most common baking ingredient, made by grinding wheat grains. Used for bread, pasta, cakes and sauces. Contains gluten which gives dough its elasticity.',
  description_pl = 'Mąka pszenna to najpopularniejszy składnik do pieczenia, otrzymywany przez mielenie ziaren pszenicy. Stosowana do chleba, makaronu, ciast i sosów. Zawiera gluten nadający ciastu elastyczność.',
  description_ru = 'Пшеничная мука — самый распространённый ингредиент для выпечки, получаемый путём помола зёрен пшеницы. Используется для хлеба, макарон, тортов и соусов. Содержит глютен.',
  description_uk = 'Пшеничне борошно — найпоширеніший інгредієнт для випічки, отриманий шляхом помолу зерен пшениці. Використовується для хліба, макаронів, тортів і соусів. Містить глютен.'
WHERE LOWER(name_en) LIKE '%flour%';

UPDATE catalog_ingredients SET
  description_en = 'Butter is a dairy product made from churned cream, essential in baking and cooking. Rich in fat-soluble vitamins A, D, E and K. Adds rich flavour and tender texture to dishes.',
  description_pl = 'Masło to produkt mleczny wytwarzany z ubijanej śmietany, niezbędny w pieczeniu i gotowaniu. Bogate w witaminy A, D, E i K. Nadaje potrawom bogatym smak i delikatną teksturę.',
  description_ru = 'Сливочное масло — молочный продукт из взбитых сливок, незаменимый в выпечке и кулинарии. Богато жирорастворимыми витаминами A, D, E и K.',
  description_uk = 'Вершкове масло — молочний продукт зі збитих вершків, незамінний у випічці та кулінарії. Багате на жиророзчинні вітаміни A, D, E та K.'
WHERE LOWER(name_en) LIKE '%butter%';

UPDATE catalog_ingredients SET
  description_en = 'Chicken eggs are one of the most nutritious and versatile foods. A single egg provides high-quality protein, healthy fats, and essential vitamins B12, D and choline.',
  description_pl = 'Jaja kurze to jeden z najbardziej odżywczych i wszechstronnych produktów spożywczych. Jedno jajko dostarcza pełnowartościowego białka, zdrowych tłuszczów oraz witamin B12, D i choliny.',
  description_ru = 'Куриные яйца — один из самых питательных и универсальных продуктов питания. Одно яйцо обеспечивает высококачественным белком, полезными жирами и витаминами B12, D и холином.',
  description_uk = 'Курячі яйця — один з найбільш поживних і універсальних продуктів харчування. Одне яйце забезпечує якісним білком, корисними жирами та вітамінами B12, D і холіном.'
WHERE LOWER(name_en) LIKE '%egg%' AND LOWER(name_en) NOT LIKE '%eggplant%';

UPDATE catalog_ingredients SET
  description_en = 'Olive oil is a cornerstone of Mediterranean cuisine, cold-pressed from olives. Rich in heart-healthy monounsaturated fats and powerful antioxidants like oleocanthal.',
  description_pl = 'Oliwa z oliwek to podstawa kuchni śródziemnomorskiej, tłoczona na zimno z oliwek. Bogata w zdrowe dla serca jednonienasycone tłuszcze i przeciwutleniacze.',
  description_ru = 'Оливковое масло — основа средиземноморской кухни, производится методом холодного отжима. Богато мононенасыщенными жирами и мощными антиоксидантами.',
  description_uk = 'Оливкова олія — основа середземноморської кухні, виготовляється методом холодного пресування. Багата на мононенасичені жири та потужні антиоксиданти.'
WHERE LOWER(name_en) LIKE '%olive oil%';

UPDATE catalog_ingredients SET
  description_en = 'Garlic is one of the oldest cultivated plants with potent medicinal properties. Contains allicin — a compound with antibacterial and antifungal effects. Essential in cuisines worldwide.',
  description_pl = 'Czosnek to jedna z najstarszych uprawianych roślin o silnych właściwościach leczniczych. Zawiera allicynę — związek o działaniu antybakteryjnym i przeciwgrzybiczym.',
  description_ru = 'Чеснок — одно из старейших культурных растений с мощными лечебными свойствами. Содержит аллицин — вещество с антибактериальным и противогрибковым действием.',
  description_uk = 'Часник — одна з найдавніших культурних рослин з потужними лікувальними властивостями. Містить алліцин — речовину з антибактеріальним та протигрибковим ефектом.'
WHERE LOWER(name_en) LIKE '%garlic%';

UPDATE catalog_ingredients SET
  description_en = 'Onion is a fundamental aromatic vegetable used as a base in countless dishes worldwide. Rich in quercetin, a powerful antioxidant. Provides depth and sweetness when caramelized.',
  description_pl = 'Cebula to podstawowe warzywo aromatyczne używane jako baza w niezliczonych potrawach. Bogata w kwercetynę — silny antyoksydant. Karmelizowana nadaje potrawom głębię i słodycz.',
  description_ru = 'Лук — основной ароматический овощ, используемый как база для бесчисленных блюд. Богат кверцетином — мощным антиоксидантом. Карамелизированный придаёт блюдам глубину вкуса.',
  description_uk = 'Цибуля — основний ароматичний овоч, що використовується як база для незліченних страв. Багата на кверцетин — потужний антиоксидант. Карамелізована надає стравам глибину та солодкість.'
WHERE LOWER(name_en) LIKE '%onion%' AND LOWER(name_en) NOT LIKE '%spring%';

UPDATE catalog_ingredients SET
  description_en = 'Potato is one of the worlds most important staple crops, originating from the Andes. Excellent source of complex carbohydrates, vitamin C, potassium and dietary fiber.',
  description_pl = 'Ziemniak to jeden z najważniejszych produktów spożywczych na świecie, pochodzący z Andów. Doskonałe źródło węglowodanów złożonych, witaminy C, potasu i błonnika pokarmowego.',
  description_ru = 'Картофель — один из важнейших продовольственных культур мира, родом из Анд. Отличный источник сложных углеводов, витамина C, калия и пищевых волокон.',
  description_uk = 'Картопля — одна з найважливіших продовольчих культур світу, родом з Анд. Відмінне джерело складних вуглеводів, вітаміну C, калію та харчових волокон.'
WHERE LOWER(name_en) LIKE '%potato%' AND LOWER(name_en) NOT LIKE '%sweet%';

UPDATE catalog_ingredients SET
  description_en = 'Tomato is botanically a fruit but culinarily a vegetable, originating from Central America. Rich in lycopene — a powerful antioxidant linked to reduced cancer risk and heart health.',
  description_pl = 'Pomidor jest botanicznie owocem, ale kulinarnie warzywem, pochodzącym z Ameryki Środkowej. Bogaty w likopen — silny antyoksydant związany ze zmniejszonym ryzykiem nowotworów.',
  description_ru = 'Помидор — ботанически фрукт, но кулинарно — овощ, родом из Центральной Америки. Богат ликопином — мощным антиоксидантом, снижающим риск рака и улучшающим здоровье сердца.',
  description_uk = 'Томат — ботанічно фрукт, але кулінарно — овоч, родом з Центральної Америки. Багатий на лікопін — потужний антиоксидант, що знижує ризик раку та покращує здоров''я серця.'
WHERE LOWER(name_en) LIKE '%tomato%' AND LOWER(name_en) NOT LIKE '%paste%' AND LOWER(name_en) NOT LIKE '%sauce%';

UPDATE catalog_ingredients SET
  description_en = 'Rice is the staple grain for over half the world population. Naturally gluten-free, a good source of energy. White rice is quick to digest; brown rice retains its fiber-rich bran layer.',
  description_pl = 'Ryż to podstawowe zboże dla ponad połowy światowej populacji. Naturalnie bezglutenowy, dobre źródło energii. Biały ryż szybko się trawi; brązowy zachowuje warstwę otrębów bogatą w błonnik.',
  description_ru = 'Рис — основная зерновая культура для более чем половины населения мира. Натурально безглютеновый, хороший источник энергии. Белый рис быстро усваивается; коричневый сохраняет отруби.',
  description_uk = 'Рис — основна зернова культура для більш ніж половини населення світу. Природно безглютеновий, добре джерело енергії. Білий рис швидко засвоюється; коричневий зберігає висівки.'
WHERE LOWER(name_en) LIKE '%rice%' AND LOWER(name_en) NOT LIKE '%vinegar%';

UPDATE catalog_ingredients SET
  description_en = 'Milk is a nutrient-rich liquid produced by mammals, a cornerstone of dairy nutrition. An excellent source of calcium, protein, vitamin B12 and phosphorus for bone health.',
  description_pl = 'Mleko to bogaty w składniki odżywcze płyn produkowany przez ssaki. Doskonałe źródło wapnia, białka, witaminy B12 i fosforu niezbędnych dla zdrowia kości.',
  description_ru = 'Молоко — богатая питательными веществами жидкость, производимая млекопитающими. Отличный источник кальция, белка, витамина B12 и фосфора для здоровья костей.',
  description_uk = 'Молоко — багата на поживні речовини рідина, що виробляється ссавцями. Відмінне джерело кальцію, білка, вітаміну B12 та фосфору для здоров''я кісток.'
WHERE LOWER(name_en) LIKE '%milk%' AND LOWER(name_en) NOT LIKE '%coconut%';

UPDATE catalog_ingredients SET
  description_en = 'Honey is a natural sweetener produced by bees from flower nectar. Contains enzymes, antioxidants and trace minerals. Has antimicrobial properties and a lower glycemic index than refined sugar.',
  description_pl = 'Miód to naturalny słodzik wytwarzany przez pszczoły z nektaru kwiatowego. Zawiera enzymy, antyoksydanty i mikroelementy. Ma właściwości przeciwbakteryjne i niższy indeks glikemiczny niż cukier.',
  description_ru = 'Мёд — натуральный подсластитель, производимый пчёлами из цветочного нектара. Содержит ферменты, антиоксиданты и микроэлементы. Обладает антимикробными свойствами.',
  description_uk = 'Мед — натуральний підсолоджувач, що виробляється бджолами з квіткового нектару. Містить ферменти, антиоксиданти та мікроелементи. Має антимікробні властивості.'
WHERE LOWER(name_en) LIKE '%honey%';

UPDATE catalog_ingredients SET
  description_en = 'Avocado is a creamy fruit native to Central America, prized for its healthy monounsaturated fats. Rich in potassium (more than bananas), folate, and vitamins K, C and B6.',
  description_pl = 'Awokado to kremowy owoc pochodzący z Ameryki Środkowej, ceniony za zdrowe jednonienasycone tłuszcze. Bogate w potas (więcej niż banany), folian i witaminy K, C i B6.',
  description_ru = 'Авокадо — кремовый фрукт родом из Центральной Америки, ценимый за полезные мононенасыщенные жиры. Богато калием (больше, чем бананы), фолатом и витаминами K, C и B6.',
  description_uk = 'Авокадо — вершковий фрукт родом з Центральної Америки, цінований за корисні мононенасичені жири. Багате на калій (більше, ніж банани), фолат і вітаміни K, C та B6.'
WHERE LOWER(name_en) LIKE '%avocado%';

UPDATE catalog_ingredients SET
  description_en = 'Lemon is a citrus fruit celebrated for its bright acidity and high vitamin C content. The juice and zest add freshness to both sweet and savoury dishes and help balance flavours.',
  description_pl = 'Cytryna to owoc cytrusowy ceniony za orzeźwiającą kwasowość i wysoką zawartość witaminy C. Sok i skórka dodają świeżości zarówno słodkim, jak i wytrawnym potrawom.',
  description_ru = 'Лимон — цитрусовый фрукт, известный яркой кислотностью и высоким содержанием витамина C. Сок и цедра добавляют свежесть как сладким, так и солёным блюдам.',
  description_uk = 'Лимон — цитрусовий фрукт, відомий яскравою кислотністю та високим вмістом вітаміну C. Сік та цедра додають свіжості як солодким, так і пікантним стравам.'
WHERE LOWER(name_en) LIKE '%lemon%';

UPDATE catalog_ingredients SET
  description_en = 'Carrot is a root vegetable exceptionally rich in beta-carotene, which the body converts to vitamin A. Supports eye health, immune function and gives a natural sweetness to soups and stews.',
  description_pl = 'Marchew to warzywo korzeniowe wyjątkowo bogate w beta-karoten, który organizm przekształca w witaminę A. Wspiera zdrowie oczu, funkcje odpornościowe i nadaje naturalną słodycz zupom.',
  description_ru = 'Морковь — корнеплод, исключительно богатый бета-каротином, который организм превращает в витамин A. Поддерживает здоровье глаз и иммунную функцию.',
  description_uk = 'Морква — коренеплід, надзвичайно багатий на бета-каротин, який організм перетворює на вітамін A. Підтримує здоров''я очей та імунну функцію.'
WHERE LOWER(name_en) LIKE '%carrot%';
