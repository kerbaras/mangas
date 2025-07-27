package integrations

import "github.com/kerbaras/mangas/pkg/data"

type Processor interface {
	Process(chapter data.Chapter, image []byte) error
}
