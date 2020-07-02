project(resvgthumbnailer)

cmake_minimum_required(VERSION 2.8.12 FATAL_ERROR)

find_package(ECM 1.0.0 REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH} ${ECM_KDE_MODULE_DIR})

include(FeatureSummary)
include(WriteBasicConfigVersionFile)
include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)

find_package(KF5 REQUIRED COMPONENTS KIO I18n Config)

include_directories(
    ${CMAKE_CURRENT_BINARY_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../../c-api
)

set(resvgthumbnailer_PART_SRCS
    resvgthumbnailer.cpp
)

add_library(resvgthumbnailer MODULE ${resvgthumbnailer_PART_SRCS})

target_link_libraries(resvgthumbnailer KF5::KIOWidgets KF5::KIOCore KF5::I18n KF5::ConfigCore KF5::ConfigGui resvg)

install(TARGETS resvgthumbnailer DESTINATION ${PLUGIN_INSTALL_DIR})

########### install files ###############

install(FILES resvgthumbnailer.desktop DESTINATION ${SERVICES_INSTALL_DIR})
