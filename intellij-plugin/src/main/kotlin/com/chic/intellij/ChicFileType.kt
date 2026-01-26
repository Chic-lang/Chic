package com.chic.intellij

import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

class ChicFileType : LanguageFileType(ChicLanguage) {
    override fun getName(): String = "Chic"

    override fun getDescription(): String = "Chic source file"

    override fun getDefaultExtension(): String = "cl"

    override fun getIcon(): Icon? = null

    companion object {
        @JvmField
        val INSTANCE = ChicFileType()
    }
}

