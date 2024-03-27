project(svgrthumbnailer)

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
    ${CMAKE_CURRENT_SOURCE_DIR}/../../crates/c-api
)

set(svgrthumbnailer_PART_SRCS
    svgrthumbnailer.cpp
)

add_library(svgrthumbnailer MODULE ${svgrthumbnailer_PART_SRCS})

target_link_libraries(svgrthumbnailer KF5::KIOWidgets KF5::KIOCore KF5::I18n KF5::ConfigCore KF5::ConfigGui svgr)

install(TARGETS svgrthumbnailer DESTINATION ${PLUGIN_INSTALL_DIR})

########### install files ###############

install(FILES svgrthumbnailer.desktop DESTINATION ${SERVICES_INSTALL_DIR})
