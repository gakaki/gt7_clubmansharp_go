package com.gakaki.gt7offline

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items as gridItems
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.MenuBook
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.DirectionsCar
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PhotoLibrary
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Divider
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

private enum class GroupMode(val zh: String, val en: String) {
    Brand("按厂商", "Manufacturer"),
    Country("按国家", "Country"),
    Drivetrain("按驱动", "Drivetrain"),
    Power("按马力", "Power"),
    Weight("按车重", "Weight"),
}

private enum class SourceFilter(val id: String, val zh: String, val en: String) {
    All("all", "全部", "All"),
    BrandCentral("brandCentral", "品牌店", "Brand Central"),
    UsedCar("usedCar", "二手车", "Used Cars"),
    Collection("vehicleCollection", "车辆收藏", "Car Collection"),
    Cafe("cafe", "咖啡屋", "Cafe"),
}

private enum class DetailTab(val zh: String, val en: String) {
    Overview("官方介绍", "Overview"),
    Website("GT7网站介绍", "GT7 website"),
    Cafe("咖啡屋对白", "Cafe"),
    BrandCentral("品牌购物", "Brand Central"),
    VehicleCollection("车辆收藏", "Car Collection"),
    Garage("车库数据", "Garage"),
    UsedCar("二手车", "Used Cars"),
    Hagerty("收藏馆", "Hagerty"),
    Gallery("4K图片", "Gallery"),
}

@Composable
fun Gt7OfflineApp() {
    Gt7Theme {
        val repository = remember { OfflineRepository() }
        var cars by remember { mutableStateOf<List<Car>?>(null) }
        var menuBooks by remember { mutableStateOf<List<MenuBook>>(emptyList()) }
        var loadError by remember { mutableStateOf<String?>(null) }
        var selectedCarId by remember { mutableStateOf<String?>(null) }
        var selectedMenuId by remember { mutableStateOf<String?>(null) }
        var menuArchiveOpen by remember { mutableStateOf(false) }
        var locale by remember { mutableStateOf("cn") }
        var mirror by remember { mutableStateOf<MirrorCatalog?>(null) }
        var dialogueTranslations by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
        var collectionTranslations by remember {
            mutableStateOf(VehicleCollectionLanguage())
        }

        LaunchedEffect(Unit) {
            runCatching {
                cars = repository.loadCars()
                menuBooks = repository.loadMenuBooks()
            }.onFailure {
                loadError = it.stackTraceToString()
            }
        }
        LaunchedEffect(locale) {
            runCatching {
                mirror = repository.loadMirror(locale)
                dialogueTranslations = repository.loadDialogueTranslations(locale)
                collectionTranslations = repository.loadCollectionTranslations(locale)
            }
        }

        when {
            loadError != null -> ErrorScreen(loadError.orEmpty())
            cars == null -> LoadingScreen()
            else -> {
                val catalog = cars.orEmpty()
                val selectedCar = catalog.firstOrNull { it.id == selectedCarId }
                val selectedMenu = menuBooks.firstOrNull { it.id == selectedMenuId }
                if (selectedCar != null) {
                    CarDetailScreen(
                        repository = repository,
                        car = selectedCar,
                        locale = locale,
                        onLocaleChange = { locale = it },
                        mirror = mirror,
                        dialogueTranslations = dialogueTranslations,
                        collectionTranslations = collectionTranslations,
                        onBack = { selectedCarId = null },
                    )
                } else if (selectedMenu != null) {
                    MenuBookDetailScreen(
                        repository = repository,
                        menu = selectedMenu,
                        locale = locale,
                        onBack = { selectedMenuId = null },
                    )
                } else {
                    Scaffold(
                        topBar = {
                            MainTopBar(
                                repository = repository,
                                locale = locale,
                                onLocaleChange = { locale = it },
                                menuArchiveOpen = menuArchiveOpen,
                                onToggleArchive = {
                                    menuArchiveOpen = !menuArchiveOpen
                                    selectedMenuId = null
                                },
                            )
                        },
                        containerColor = Color(0xFF0B0B0D),
                    ) { padding ->
                        if (menuArchiveOpen) {
                            MenuBookArchive(
                                repository = repository,
                                menus = menuBooks,
                                locale = locale,
                                modifier = Modifier.padding(padding),
                                onOpen = { selectedMenuId = it },
                            )
                        } else {
                            CatalogScreen(
                                repository = repository,
                                cars = catalog,
                                locale = locale,
                                modifier = Modifier.padding(padding),
                                onOpenCar = { selectedCarId = it },
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LoadingScreen() {
    Box(Modifier.fillMaxSize().background(Color(0xFF0B0B0D)), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text("GT7", style = MaterialTheme.typography.displaySmall)
            Text("OFFLINE ARCHIVE", color = Color(0xFF8E8E96), fontWeight = FontWeight.Bold)
            Spacer(Modifier.height(18.dp))
            Text("正在读取离线车辆档案…", color = Color(0xFFB8B8C0))
        }
    }
}

@Composable
private fun ErrorScreen(message: String) {
    Box(Modifier.fillMaxSize().background(Color(0xFF0B0B0D)).padding(24.dp)) {
        Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("离线资源读取失败", style = MaterialTheme.typography.headlineMedium)
            Text(message, color = Color(0xFFFF8C96))
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MainTopBar(
    repository: OfflineRepository,
    locale: String,
    onLocaleChange: (String) -> Unit,
    menuArchiveOpen: Boolean,
    onToggleArchive: () -> Unit,
) {
    TopAppBar(
        title = {
            Row(verticalAlignment = Alignment.CenterVertically) {
                OfflineImage(
                    repository = repository,
                    path = "common/dist/gt7/carlist/assets/logo_gt7-BQSHSLc9.png",
                    contentDescription = "Gran Turismo 7",
                    modifier = Modifier.width(118.dp).height(38.dp),
                )
                Spacer(Modifier.width(14.dp))
                Column {
                    Text("GT7 OFFLINE ARCHIVE", fontWeight = FontWeight.Black)
                    Text(
                        "ALL GAME DATA EMBEDDED · NO NETWORK",
                        style = MaterialTheme.typography.labelSmall,
                        color = Color(0xFF85858D),
                    )
                }
            }
        },
        actions = {
            LanguageMenu(locale, onLocaleChange)
            TextButton(onClick = onToggleArchive) {
                Icon(Icons.AutoMirrored.Filled.MenuBook, contentDescription = null)
                Spacer(Modifier.width(6.dp))
                Text(if (menuArchiveOpen) "返回车辆" else "菜单书档案")
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(
            containerColor = Color(0xFF0B0B0D),
            titleContentColor = Color.White,
            actionIconContentColor = Color.White,
        ),
    )
}

@Composable
private fun LanguageMenu(locale: String, onLocaleChange: (String) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    val current = offlineLanguages.firstOrNull { it.code == locale }
    Box {
        TextButton(onClick = { expanded = true }) {
            Icon(Icons.Default.Language, contentDescription = null)
            Spacer(Modifier.width(6.dp))
            Text(current?.label ?: locale)
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            offlineLanguages.forEach { language ->
                DropdownMenuItem(
                    text = {
                        Text(
                            language.label,
                            fontWeight = if (language.code == locale) FontWeight.Bold else FontWeight.Normal,
                        )
                    },
                    onClick = {
                        onLocaleChange(language.code)
                        expanded = false
                    },
                )
            }
        }
    }
}

@Composable
private fun CatalogScreen(
    repository: OfflineRepository,
    cars: List<Car>,
    locale: String,
    modifier: Modifier = Modifier,
    onOpenCar: (String) -> Unit,
) {
    var query by remember { mutableStateOf("") }
    var groupMode by remember { mutableStateOf(GroupMode.Brand) }
    var source by remember { mutableStateOf(SourceFilter.All) }
    var brand by remember { mutableStateOf("ALL") }
    val brands = remember(cars) {
        cars.groupBy { it.brand }.mapValues { it.value.size }.toList()
            .sortedBy { it.first.lowercase() }
    }
    val filtered = remember(cars, query, source, brand) {
        cars.filter { car ->
            val textMatch = query.isBlank() ||
                listOf(car.nameEn, car.nameZh, car.brand).any {
                    it.contains(query, ignoreCase = true)
                } ||
                car.conversation.any {
                    listOfNotNull(it.en, it.zh, it.speaker, it.speakerZh)
                        .any { value -> value.contains(query, ignoreCase = true) }
                }
            val brandMatch = brand == "ALL" || car.brand == brand
            val sourceMatch = when (source) {
                SourceFilter.All -> true
                SourceFilter.Cafe -> car.conversation.isNotEmpty() || car.dialogues.isNotEmpty()
                else -> car.texts.containsKey(source.id)
            }
            textMatch && brandMatch && sourceMatch
        }
    }
    val groups = remember(filtered, groupMode, locale) { groupCars(filtered, groupMode, locale) }

    BoxWithConstraints(modifier.fillMaxSize()) {
        val wide = maxWidth >= 960.dp
        Row(Modifier.fillMaxSize()) {
            if (wide) {
                BrandRail(
                    brands = brands,
                    selected = brand,
                    onSelect = { brand = it },
                )
            }
            Column(Modifier.weight(1f).fillMaxHeight()) {
                CatalogControls(
                    query = query,
                    onQueryChange = { query = it },
                    source = source,
                    onSourceChange = { source = it },
                    groupMode = groupMode,
                    onGroupChange = { groupMode = it },
                    brands = brands,
                    selectedBrand = brand,
                    onBrandChange = { brand = it },
                    showBrandStrip = !wide,
                    total = filtered.size,
                )
                LazyVerticalGrid(
                    columns = GridCells.Adaptive(if (wide) 185.dp else 150.dp),
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 14.dp, bottom = 54.dp),
                    horizontalArrangement = Arrangement.spacedBy(14.dp),
                    verticalArrangement = Arrangement.spacedBy(20.dp),
                ) {
                    groups.forEach { group ->
                        item(span = { GridItemSpan(maxLineSpan) }, key = "heading-${group.key}") {
                            GroupHeading(group.title, group.cars.size, groupMode)
                        }
                        gridItems(group.cars, key = { it.id }) { car ->
                            CarCard(repository, car, locale) { onOpenCar(car.id) }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun BrandRail(
    brands: List<Pair<String, Int>>,
    selected: String,
    onSelect: (String) -> Unit,
) {
    LazyColumn(
        modifier = Modifier.width(218.dp).fillMaxHeight()
            .background(Color(0xFF111114))
            .padding(top = 12.dp, bottom = 24.dp),
        contentPadding = PaddingValues(horizontal = 12.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        item {
            Text(
                "MANUFACTURERS",
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                style = MaterialTheme.typography.labelSmall,
                color = Color(0xFF777780),
                fontWeight = FontWeight.Bold,
            )
        }
        item {
            BrandRow("全部品牌", brands.sumOf { it.second }, selected == "ALL") { onSelect("ALL") }
        }
        items(brands, key = { it.first }) { (name, count) ->
            BrandRow(name, count, selected == name) { onSelect(name) }
        }
    }
}

@Composable
private fun BrandRow(name: String, count: Int, selected: Boolean, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth()
            .background(if (selected) Color(0xFF2B2B30) else Color.Transparent, RoundedCornerShape(6.dp))
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 9.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            name,
            modifier = Modifier.weight(1f),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
        )
        Text(count.toString(), color = Color(0xFF85858D), style = MaterialTheme.typography.labelSmall)
    }
}

@Composable
private fun CatalogControls(
    query: String,
    onQueryChange: (String) -> Unit,
    source: SourceFilter,
    onSourceChange: (SourceFilter) -> Unit,
    groupMode: GroupMode,
    onGroupChange: (GroupMode) -> Unit,
    brands: List<Pair<String, Int>>,
    selectedBrand: String,
    onBrandChange: (String) -> Unit,
    showBrandStrip: Boolean,
    total: Int,
) {
    Column(
        Modifier.fillMaxWidth().background(Color(0xFF0B0B0D)).padding(start = 20.dp, end = 20.dp, top = 12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = query,
                onValueChange = onQueryChange,
                modifier = Modifier.weight(1f),
                singleLine = true,
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                placeholder = { Text("搜索车型、品牌、对白…") },
                shape = RoundedCornerShape(10.dp),
            )
            Column(horizontalAlignment = Alignment.End) {
                Text(total.toString(), style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Black)
                Text("车辆", color = Color(0xFF85858D), style = MaterialTheme.typography.labelSmall)
            }
        }
        LazyRow(horizontalArrangement = Arrangement.spacedBy(7.dp)) {
            items(SourceFilter.entries) { filter ->
                FilterChip(
                    selected = source == filter,
                    onClick = { onSourceChange(filter) },
                    label = { Text(if (filter == SourceFilter.All) filter.zh else filter.zh) },
                )
            }
        }
        LazyRow(horizontalArrangement = Arrangement.spacedBy(7.dp)) {
            items(GroupMode.entries) { mode ->
                FilterChip(
                    selected = groupMode == mode,
                    onClick = { onGroupChange(mode) },
                    label = { Text(mode.zh) },
                )
            }
        }
        if (showBrandStrip) {
            LazyRow(horizontalArrangement = Arrangement.spacedBy(7.dp)) {
                item {
                    FilterChip(
                        selected = selectedBrand == "ALL",
                        onClick = { onBrandChange("ALL") },
                        label = { Text("全部品牌") },
                    )
                }
                items(brands, key = { it.first }) { (name, count) ->
                    FilterChip(
                        selected = selectedBrand == name,
                        onClick = { onBrandChange(name) },
                        label = { Text("$name $count") },
                    )
                }
            }
        }
        Divider(color = Color(0xFF27272B))
    }
}

@Composable
private fun GroupHeading(title: String, count: Int, mode: GroupMode) {
    Row(
        Modifier.fillMaxWidth().padding(top = 12.dp, bottom = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            title,
            style = if (mode == GroupMode.Brand) {
                MaterialTheme.typography.titleLarge
            } else {
                MaterialTheme.typography.headlineMedium
            },
            fontWeight = FontWeight.Black,
        )
        Spacer(Modifier.width(14.dp))
        Divider(Modifier.weight(1f), color = Color(0xFF2F2F34))
        Spacer(Modifier.width(14.dp))
        Text("$count 辆", color = Color(0xFF9999A2), style = MaterialTheme.typography.labelLarge)
    }
}

@Composable
private fun CarCard(
    repository: OfflineRepository,
    car: Car,
    locale: String,
    onClick: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
        colors = CardDefaults.cardColors(containerColor = Color(0xFF161619)),
        shape = RoundedCornerShape(11.dp),
    ) {
        OfflineImage(
            repository = repository,
            path = car.image,
            contentDescription = car.nameEn,
            modifier = Modifier.fillMaxWidth().height(112.dp).padding(8.dp),
        )
        Column(Modifier.padding(start = 12.dp, end = 12.dp, bottom = 13.dp)) {
            Text(
                car.brand.uppercase(),
                color = Color(0xFF888891),
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.Bold,
            )
            Spacer(Modifier.height(3.dp))
            Text(
                car.displayName(locale),
                fontWeight = FontWeight.Bold,
                maxLines = 2,
                minLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                val dialogueCount = car.conversation.ifEmpty { car.dialogues }.size
                Badge(if (dialogueCount > 0) "对话 $dialogueCount" else "无对话", dialogueCount > 0)
                if (car.texts.isNotEmpty() || car.officialDetails != null) {
                    Badge("资料 ${(if (car.officialDetails != null) 1 else 0) + car.texts.size}", true)
                }
            }
        }
    }
}

@Composable
private fun Badge(text: String, active: Boolean) {
    Text(
        text,
        modifier = Modifier
            .background(
                if (active) Color(0xFF253B32) else Color(0xFF252529),
                RoundedCornerShape(999.dp),
            )
            .padding(horizontal = 7.dp, vertical = 3.dp),
        color = if (active) Color(0xFFA4E3C4) else Color(0xFF8E8E96),
        style = MaterialTheme.typography.labelSmall,
    )
}

private data class CarGroup(val key: String, val title: String, val cars: List<Car>)

private fun groupCars(cars: List<Car>, mode: GroupMode, locale: String): List<CarGroup> {
    val buckets = linkedMapOf<String, MutableList<Car>>()
    cars.forEach { car ->
        val key = when (mode) {
            GroupMode.Brand -> car.brand
            GroupMode.Country -> {
                val country = car.garageDetails?.country
                (if (isChinese(locale)) country?.nameZh else country?.nameEn)
                    .orEmpty()
                    .ifBlank { "其他" }
            }
            GroupMode.Drivetrain -> car.spec(locale, "driveTrain") ?: "其他"
            GroupMode.Power -> powerBucket(car.spec(locale, "power_v") ?: car.spec("us", "power_v"))
            GroupMode.Weight -> weightBucket(car.spec(locale, "weight_v") ?: car.spec("us", "weight_v"))
        }
        buckets.getOrPut(key) { mutableListOf() }.add(car)
    }
    val entries = buckets.entries.map { (key, value) ->
        CarGroup(key, key, value.sortedBy { it.displayName(locale).lowercase() })
    }
    return when (mode) {
        GroupMode.Brand -> entries.sortedBy { it.title.lowercase() }
        GroupMode.Power, GroupMode.Weight -> entries.sortedBy { it.key.toIntOrNull() ?: 999 }
        GroupMode.Drivetrain -> entries.sortedBy {
            listOf("FF", "FR", "4WD", "MR", "RR").indexOf(it.title).let { index ->
                if (index < 0) 99 else index
            }
        }
        GroupMode.Country -> entries.sortedBy { it.title }
    }
}

private fun powerBucket(raw: String?): String {
    val value = raw?.toDoubleOrNull() ?: 0.0
    val lower = listOf(1600, 1300, 1100, 1000, 900, 800, 700, 600, 500, 400, 300, 200, 100, 0)
        .firstOrNull { value >= it } ?: 0
    return lower.toString()
}

private fun weightBucket(raw: String?): String {
    val value = raw?.toDoubleOrNull() ?: 0.0
    val lower = listOf(2300, 2000, 1800, 1700, 1600, 1500, 1400, 1300, 1200, 1000, 900, 800, 700, 600, 500, 0)
        .firstOrNull { value >= it } ?: 0
    return lower.toString()
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CarDetailScreen(
    repository: OfflineRepository,
    car: Car,
    locale: String,
    onLocaleChange: (String) -> Unit,
    mirror: MirrorCatalog?,
    dialogueTranslations: Map<String, String>,
    collectionTranslations: VehicleCollectionLanguage,
    onBack: () -> Unit,
) {
    var tab by remember(car.id) { mutableStateOf(DetailTab.Overview) }
    var galleryIndex by remember(car.id) { mutableIntStateOf(0) }
    var galleryOpen by remember(car.id) { mutableStateOf(false) }
    val mirrorCar = mirror?.forCar(car)
    val tabs = remember(car) {
        buildList {
            add(DetailTab.Overview)
            add(DetailTab.Website)
            if (car.conversation.isNotEmpty() || car.dialogues.isNotEmpty()) add(DetailTab.Cafe)
            if (car.texts.containsKey("brandCentral")) add(DetailTab.BrandCentral)
            if (car.texts.containsKey("vehicleCollection")) add(DetailTab.VehicleCollection)
            if (car.specs != null) add(DetailTab.Garage)
            if (car.texts.containsKey("usedCar")) add(DetailTab.UsedCar)
            if (car.texts.containsKey("collection")) add(DetailTab.Hagerty)
            add(DetailTab.Gallery)
        }
    }
    val images = mirrorCar?.images.orEmpty().ifEmpty {
        car.officialDetails?.images.orEmpty().map { it.jxl }
    }
    Scaffold(
        topBar = {
            TopAppBar(
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
                title = {
                    Column {
                        Text(car.displayName(locale), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        Text(
                            "${car.brand.uppercase()} · ${mirrorCar?.carClass ?: car.spec(locale, "carClass").orEmpty()}",
                            style = MaterialTheme.typography.labelSmall,
                            color = Color(0xFF85858D),
                        )
                    }
                },
                actions = {
                    LanguageMenu(locale, onLocaleChange)
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Color(0xFF0B0B0D)),
            )
        },
        containerColor = Color(0xFF0B0B0D),
    ) { padding ->
        if (galleryOpen) {
            GalleryScreen(
                repository = repository,
                car = car,
                locale = locale,
                mirrorCar = mirrorCar,
                images = images,
                initialIndex = galleryIndex,
                modifier = Modifier.padding(padding),
                onIndexChange = { galleryIndex = it },
                onBack = { galleryOpen = false },
            )
            return@Scaffold
        }

        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(start = 24.dp, end = 24.dp, top = 18.dp, bottom = 64.dp),
            verticalArrangement = Arrangement.spacedBy(18.dp),
        ) {
            item {
                CarHero(
                    repository = repository,
                    car = car,
                    locale = locale,
                    mirrorCar = mirrorCar,
                    image = images.firstOrNull(),
                    onOpenGallery = {
                        galleryIndex = 0
                        galleryOpen = true
                    },
                )
            }
            item {
                LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    items(tabs) { item ->
                        FilterChip(
                            selected = tab == item,
                            onClick = { tab = item },
                            label = { Text(item.zh) },
                        )
                    }
                }
            }
            item {
                when (tab) {
                    DetailTab.Overview -> OverviewTab(car, locale)
                    DetailTab.Website -> WebsiteTab(mirrorCar)
                    DetailTab.Cafe -> CafeTab(
                        repository,
                        car,
                        locale,
                        dialogueTranslations,
                    )
                    DetailTab.BrandCentral,
                    DetailTab.VehicleCollection,
                    DetailTab.UsedCar,
                    DetailTab.Hagerty,
                    -> TextTab(
                        car = car,
                        tab = tab,
                        locale = locale,
                        collectionTranslations = collectionTranslations,
                    )
                    DetailTab.Garage -> GarageTab(car, locale)
                    DetailTab.Gallery -> GalleryTab(
                        repository = repository,
                        images = images,
                        fallback = car.image,
                        onOpen = { index ->
                            galleryIndex = index
                            galleryOpen = true
                        },
                    )
                }
            }
        }
    }
}

@Composable
private fun CarHero(
    repository: OfflineRepository,
    car: Car,
    locale: String,
    mirrorCar: MirrorCar?,
    image: String?,
    onOpenGallery: () -> Unit,
) {
    Card(
        colors = CardDefaults.cardColors(containerColor = Color(0xFF151518)),
        shape = RoundedCornerShape(14.dp),
    ) {
        BoxWithConstraints {
            val wide = maxWidth >= 760.dp
            if (wide) {
                Row(Modifier.fillMaxWidth().height(390.dp)) {
                    OfflineImage(
                        repository = repository,
                        path = image,
                        fallbackPath = car.image,
                        contentDescription = car.nameEn,
                        modifier = Modifier.weight(1.4f).fillMaxHeight(),
                        contentScale = ContentScale.Crop,
                    )
                    HeroCopy(repository, car, locale, mirrorCar, onOpenGallery, Modifier.weight(1f))
                }
            } else {
                Column {
                    OfflineImage(
                        repository = repository,
                        path = image,
                        fallbackPath = car.image,
                        contentDescription = car.nameEn,
                        modifier = Modifier.fillMaxWidth().height(250.dp),
                        contentScale = ContentScale.Crop,
                    )
                    HeroCopy(repository, car, locale, mirrorCar, onOpenGallery, Modifier.fillMaxWidth())
                }
            }
        }
    }
}

@Composable
private fun HeroCopy(
    repository: OfflineRepository,
    car: Car,
    locale: String,
    mirrorCar: MirrorCar?,
    onOpenGallery: () -> Unit,
    modifier: Modifier,
) {
    Column(
        modifier.fillMaxHeight().padding(24.dp),
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
            OfflineImage(
                repository = repository,
                path = mirrorCar?.logo,
                fallbackPath = car.brandLogo?.let { "logos/$it" },
                contentDescription = car.brand,
                modifier = Modifier.width(96.dp).height(48.dp),
            )
            Text(car.displayName(locale), style = MaterialTheme.typography.headlineMedium)
            Text(
                listOfNotNull(
                    mirrorCar?.carClass?.ifBlank { null },
                    mirrorCar?.performancePoint?.ifBlank { null },
                    car.spec(locale, "driveTrain"),
                ).joinToString(" · "),
                color = Color(0xFFA8A8B0),
                fontWeight = FontWeight.Bold,
            )
            BodyCopy(localizedOfficialHero(car, locale))
        }
        Button(onClick = onOpenGallery) {
            Icon(Icons.Default.PhotoLibrary, contentDescription = null)
            Spacer(Modifier.width(8.dp))
            Text("全屏查看 4K 图片")
        }
    }
}

@Composable
private fun OverviewTab(car: Car, locale: String) {
    ContentSection("官方车表介绍") {
        BodyCopy(localizedOfficialHero(car, locale))
        Spacer(Modifier.height(14.dp))
        BodyCopy(localizedOfficialDescription(car, locale))
        Spacer(Modifier.height(18.dp))
        SourceLine(car.officialDetails?.officialId?.let { "OFFICIAL · $it" } ?: "OFFICIAL CAR LIST")
        Spacer(Modifier.height(18.dp))
        val fields = listOf(
            "performancePoint" to "性能点",
            "maxPower" to "最大马力",
            "maxTorque" to "最大扭矩",
            "weight" to "车重",
            "driveTrain" to "驱动",
            "aspirationLong" to "进气",
            "displacement" to "排气量",
            "length" to "全长",
            "width" to "全宽",
            "height" to "全高",
        )
        fields.chunked(2).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { (key, label) ->
                    SpecPill(
                        label,
                        car.spec(locale, key) ?: "—",
                        Modifier.weight(1f),
                    )
                }
                if (row.size == 1) Spacer(Modifier.weight(1f))
            }
            Spacer(Modifier.height(8.dp))
        }
    }
}

@Composable
private fun WebsiteTab(mirrorCar: MirrorCar?) {
    ContentSection("GT7网站介绍") {
        if (mirrorCar == null) {
            Text("此车辆没有网站镜像介绍。", color = Color(0xFF9B9BA3))
            return@ContentSection
        }
        BodyCopy(mirrorCar.hero)
        Spacer(Modifier.height(14.dp))
        BodyCopy(mirrorCar.description)
        Spacer(Modifier.height(18.dp))
        mirrorCar.specs.chunked(2).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { spec ->
                    SpecPill(spec.label, spec.value, Modifier.weight(1f))
                }
                if (row.size == 1) Spacer(Modifier.weight(1f))
            }
            Spacer(Modifier.height(8.dp))
        }
    }
}

@Composable
private fun CafeTab(
    repository: OfflineRepository,
    car: Car,
    locale: String,
    translations: Map<String, String>,
) {
    val turns = (car.conversation.ifEmpty { car.dialogues })
        .distinctBy { "${it.id}|${normalizeDialogue(it.en)}|${normalizeDialogue(it.zh)}" }
    val scenes = turns.groupBy { it.sceneKey ?: it.sceneId ?: "cafe" }
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        scenes.entries.forEachIndexed { sceneIndex, (_, sceneTurns) ->
            ContentSection("咖啡屋场景 ${sceneIndex + 1} · ${sceneTurns.size} 段") {
                sceneTurns.forEachIndexed { index, turn ->
                    DialogueRow(repository, turn, locale, translations)
                    if (index < sceneTurns.lastIndex) {
                        Spacer(Modifier.height(16.dp))
                    }
                }
            }
        }
    }
}

@Composable
private fun DialogueRow(
    repository: OfflineRepository,
    turn: DialogueTurn,
    locale: String,
    translations: Map<String, String>,
) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        DialogueAvatar(repository, turn.avatar, turn.speaker)
        Column(Modifier.weight(1f)) {
            Text(
                if (isChinese(locale)) turn.speakerZh ?: turn.speaker else turn.speaker,
                fontWeight = FontWeight.Bold,
            )
            Text(
                if (isChinese(locale)) turn.roleZh.orEmpty() else turn.roleEn.orEmpty(),
                style = MaterialTheme.typography.labelSmall,
                color = Color(0xFF7F7F88),
            )
            Spacer(Modifier.height(7.dp))
            BodyCopy(dialogueText(turn, locale, translations))
        }
    }
}

private fun dialogueText(
    turn: DialogueTurn,
    locale: String,
    translations: Map<String, String>,
): String = when {
    isChinese(locale) -> turn.zh?.ifBlank { null } ?: turn.en.orEmpty()
    locale == "us" || locale == "gb" -> turn.en?.ifBlank { null } ?: turn.zh.orEmpty()
    else -> translations[turn.id]?.ifBlank { null } ?: turn.en.orEmpty()
}

private fun normalizeDialogue(text: String?): String =
    text.orEmpty().lowercase().filter { it.isLetterOrDigit() }

@Composable
private fun TextTab(
    car: Car,
    tab: DetailTab,
    locale: String,
    collectionTranslations: VehicleCollectionLanguage,
) {
    val key = when (tab) {
        DetailTab.BrandCentral -> "brandCentral"
        DetailTab.VehicleCollection -> "vehicleCollection"
        DetailTab.UsedCar -> "usedCar"
        DetailTab.Hagerty -> "collection"
        else -> return
    }
    val text = car.texts[key] ?: return
    val longId = text.textIds?.long ?: text.long?.textId
    val translatedLong = longId?.let {
        collectionTranslations.DescriptionLong[it]
    }
    ContentSection(tab.zh) {
        Text(
            if (isChinese(locale)) text.titleZh.orEmpty() else text.titleEn.orEmpty(),
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Black,
        )
        Spacer(Modifier.height(14.dp))
        BodyCopy(localizedText(text.catchCopy, locale).orEmpty())
        Spacer(Modifier.height(14.dp))
        BodyCopy(translatedLong ?: localizedText(text.long, locale).orEmpty())
        Spacer(Modifier.height(14.dp))
        BodyCopy(localizedText(text, locale).orEmpty())
        Spacer(Modifier.height(18.dp))
        SourceLine(text.textId?.let { "GAME TEXT · $it" } ?: "LOCAL GAME TEXT")
    }
}

@Composable
private fun GarageTab(car: Car, locale: String) {
    val fields = listOf(
        "performancePoint" to "性能点",
        "maxPower" to "最大马力",
        "maxTorque" to "最大扭矩",
        "weight" to "车重",
        "driveTrain" to "驱动",
        "aspirationLong" to "进气",
        "displacement" to "排气量",
        "carClass" to "车辆级别",
        "length" to "长度",
        "width" to "宽度",
        "height" to "高度",
    )
    ContentSection("车库数据") {
        fields.chunked(2).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { (key, label) ->
                    SpecPill(label, car.spec(locale, key) ?: "—", Modifier.weight(1f))
                }
                if (row.size == 1) Spacer(Modifier.weight(1f))
            }
            Spacer(Modifier.height(8.dp))
        }
        Spacer(Modifier.height(10.dp))
        val country = car.garageDetails?.country
        Text(
            "国家/地区：" + if (isChinese(locale)) country?.nameZh.orEmpty() else country?.nameEn.orEmpty(),
            fontWeight = FontWeight.Bold,
        )
        Text(
            car.garageDetails?.vehicleTypes.orEmpty()
                .joinToString("  ") { if (isChinese(locale)) it.tagZh.orEmpty() else it.tagEn.orEmpty() },
            color = Color(0xFFB5B5BC),
        )
    }
}

@Composable
private fun GalleryTab(
    repository: OfflineRepository,
    images: List<String>,
    fallback: String?,
    onOpen: (Int) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        SectionLabel("4K GALLERY · ${images.size} IMAGES")
        images.chunked(2).forEachIndexed { rowIndex, row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                row.forEachIndexed { columnIndex, image ->
                    val index = rowIndex * 2 + columnIndex
                    OfflineImage(
                        repository = repository,
                        path = image,
                        fallbackPath = fallback,
                        contentDescription = "Image ${index + 1}",
                        modifier = Modifier.weight(1f).height(190.dp).clickable { onOpen(index) },
                        contentScale = ContentScale.Crop,
                    )
                }
                if (row.size == 1) Spacer(Modifier.weight(1f))
            }
        }
    }
}

@Composable
private fun GalleryScreen(
    repository: OfflineRepository,
    car: Car,
    locale: String,
    mirrorCar: MirrorCar?,
    images: List<String>,
    initialIndex: Int,
    modifier: Modifier = Modifier,
    onIndexChange: (Int) -> Unit,
    onBack: () -> Unit,
) {
    var index by remember(car.id) { mutableIntStateOf(initialIndex.coerceAtLeast(0)) }
    var playing by remember { mutableStateOf(true) }
    var speed by remember { mutableIntStateOf(6) }
    LaunchedEffect(playing, index, speed, images.size) {
        if (playing && images.size > 1) {
            delay((5000L / speed).coerceAtLeast(300L))
            index = (index + 1) % images.size
            onIndexChange(index)
        }
    }
    BoxWithConstraints(modifier.fillMaxSize().background(Color.Black)) {
        val wide = maxWidth >= 900.dp
        if (wide) {
            Row(Modifier.fillMaxSize()) {
                GalleryCopy(
                    car = car,
                    locale = locale,
                    mirrorCar = mirrorCar,
                    modifier = Modifier.width(390.dp).fillMaxHeight(),
                )
                GalleryCanvas(
                    repository = repository,
                    image = images.getOrNull(index),
                    fallback = car.image,
                    modifier = Modifier.weight(1f).fillMaxHeight(),
                )
            }
        } else {
            Column(Modifier.fillMaxSize()) {
                GalleryCanvas(
                    repository = repository,
                    image = images.getOrNull(index),
                    fallback = car.image,
                    modifier = Modifier.weight(1f).fillMaxWidth(),
                )
                GalleryCopy(car, locale, mirrorCar, Modifier.fillMaxWidth().height(210.dp))
            }
        }
        Surface(
            modifier = Modifier.align(Alignment.BottomCenter).padding(16.dp),
            color = Color(0xE61B1B1F),
            shape = RoundedCornerShape(14.dp),
        ) {
            Column(
                Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    TextButton(onClick = {
                        if (images.isNotEmpty()) {
                            index = (index - 1 + images.size) % images.size
                            onIndexChange(index)
                        }
                    }) { Text("←") }
                    TextButton(onClick = { playing = !playing }) {
                        Icon(
                            if (playing) Icons.Default.Pause else Icons.Default.PlayArrow,
                            contentDescription = null,
                        )
                    }
                    TextButton(onClick = {
                        if (images.isNotEmpty()) {
                            index = (index + 1) % images.size
                            onIndexChange(index)
                        }
                    }) { Text("→") }
                    Text("${index + 1} / ${images.size}", color = Color(0xFFC1C1C7))
                }
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Text("速度", style = MaterialTheme.typography.labelSmall, color = Color(0xFF85858D))
                    LazyRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        items(10) { itemIndex ->
                            val value = itemIndex + 1
                            Text(
                                "${value}×",
                                modifier = Modifier
                                    .background(
                                        if (speed == value) Color.White else Color(0xFF333338),
                                        RoundedCornerShape(4.dp),
                                    )
                                    .clickable { speed = value }
                                    .padding(horizontal = 7.dp, vertical = 4.dp),
                                color = if (speed == value) Color.Black else Color.White,
                                style = MaterialTheme.typography.labelSmall,
                            )
                        }
                    }
                }
            }
        }
        IconButton(
            onClick = onBack,
            modifier = Modifier.align(Alignment.TopEnd).padding(16.dp)
                .background(Color.White, CircleShape),
        ) {
            Icon(Icons.Default.Close, contentDescription = "关闭", tint = Color.Black)
        }
    }
}

@Composable
private fun GalleryCanvas(
    repository: OfflineRepository,
    image: String?,
    fallback: String?,
    modifier: Modifier,
) {
    OfflineImage(
        repository = repository,
        path = image,
        fallbackPath = fallback,
        contentDescription = "4K vehicle image",
        modifier = modifier,
        contentScale = ContentScale.Fit,
    )
}

@Composable
private fun GalleryCopy(
    car: Car,
    locale: String,
    mirrorCar: MirrorCar?,
    modifier: Modifier,
) {
    Column(
        modifier.background(Color(0xF20A0A0C)).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(car.displayName(locale), style = MaterialTheme.typography.headlineMedium)
        Text(
            "${mirrorCar?.carClass.orEmpty()}  ${mirrorCar?.performancePoint.orEmpty()}",
            fontWeight = FontWeight.Bold,
        )
        BodyCopy(mirrorCar?.hero.orEmpty())
        BodyCopy(mirrorCar?.description.orEmpty())
    }
}

@Composable
private fun ContentSection(title: String, content: @Composable ColumnScope.() -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = Color(0xFF151518)),
        shape = RoundedCornerShape(12.dp),
    ) {
        Column(Modifier.fillMaxWidth().padding(22.dp)) {
            SectionLabel(title)
            Spacer(Modifier.height(12.dp))
            content()
        }
    }
}

@Composable
private fun MenuBookArchive(
    repository: OfflineRepository,
    menus: List<MenuBook>,
    locale: String,
    modifier: Modifier = Modifier,
    onOpen: (String) -> Unit,
) {
    Column(modifier.fillMaxSize().padding(horizontal = 22.dp)) {
        Row(
            Modifier.fillMaxWidth().padding(vertical = 18.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.Bottom,
        ) {
            Column {
                Text("菜单书档案", style = MaterialTheme.typography.headlineMedium)
                Text("MENU BOOKS · ${menus.size} RECORDS", color = Color(0xFF85858D))
            }
            Text("咖啡屋故事、车辆与奖励均为离线数据", color = Color(0xFF85858D))
        }
        LazyVerticalGrid(
            columns = GridCells.Adaptive(190.dp),
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(bottom = 48.dp),
            horizontalArrangement = Arrangement.spacedBy(14.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            gridItems(menus, key = { it.id }) { menu ->
                Card(
                    modifier = Modifier.clickable { onOpen(menu.id) },
                    colors = CardDefaults.cardColors(containerColor = Color(0xFF17171B)),
                ) {
                    OfflineImage(
                        repository,
                        path = menu.cover,
                        contentDescription = menu.title(locale),
                        modifier = Modifier.fillMaxWidth().height(190.dp),
                        contentScale = ContentScale.Crop,
                    )
                    Column(Modifier.padding(12.dp)) {
                        Text(menu.displayNumber, color = Color(0xFF9999A1), style = MaterialTheme.typography.labelSmall)
                        Text(menu.title(locale), fontWeight = FontWeight.Bold, maxLines = 2)
                        Text(menu.collectionNameZh, color = Color(0xFF888891), maxLines = 1)
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MenuBookDetailScreen(
    repository: OfflineRepository,
    menu: MenuBook,
    locale: String,
    onBack: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                    }
                },
                title = { Text("${menu.displayNumber} · ${menu.title(locale)}") },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Color(0xFF0B0B0D)),
            )
        },
        containerColor = Color(0xFF0B0B0D),
    ) { padding ->
        LazyColumn(
            Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            item {
                OfflineImage(
                    repository,
                    path = menu.cover,
                    contentDescription = menu.title(locale),
                    modifier = Modifier.fillMaxWidth().height(360.dp),
                    contentScale = ContentScale.Crop,
                )
            }
            item {
                ContentSection(menu.categoryZh.ifBlank { menu.categoryEn }) {
                    Text(menu.title(locale), style = MaterialTheme.typography.headlineMedium)
                    Text(
                        if (isChinese(locale)) menu.collectionNameZh else menu.collectionNameEn,
                        color = Color(0xFFA6A6AE),
                    )
                    Spacer(Modifier.height(18.dp))
                    BodyCopy(if (isChinese(locale)) menu.storyZh else menu.storyEn)
                    Spacer(Modifier.height(16.dp))
                    BodyCopy(if (isChinese(locale)) menu.descriptionZh else menu.descriptionEn)
                    Spacer(Modifier.height(16.dp))
                    SourceLine("REQUIREMENTS")
                    BodyCopy(if (isChinese(locale)) menu.requirementsZh else menu.requirementsEn)
                    Spacer(Modifier.height(12.dp))
                    SourceLine("REWARD")
                    BodyCopy(if (isChinese(locale)) menu.rewardZh else menu.rewardEn)
                }
            }
            item {
                Text("COLLECTION CARS · ${menu.cars.size}", style = MaterialTheme.typography.titleLarge)
            }
            items(menu.cars, key = { it.id }) { car ->
                Card(colors = CardDefaults.cardColors(containerColor = Color(0xFF17171B))) {
                    Row(Modifier.fillMaxWidth().padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        OfflineImage(
                            repository,
                            path = car.image,
                            contentDescription = car.nameEn,
                            modifier = Modifier.width(150.dp).height(84.dp),
                        )
                        Spacer(Modifier.width(14.dp))
                        Column {
                            Text(car.brand, color = Color(0xFF888891), style = MaterialTheme.typography.labelSmall)
                            Text(if (isChinese(locale)) car.nameZh else car.nameEn, fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }
        }
    }
}
